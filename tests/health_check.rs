use emailer::config::{read_config, Config};
use emailer::email_client::EmailClient;
use emailer::telemetry::init_logging;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_logging("test", "debug", std::io::stdout);
    } else {
        init_logging("test", "debug", std::io::sink);
    }
});

async fn spawn_app() -> (String, PgPool) {
    Lazy::force(&TRACING);

    let mut config = read_config().expect("Failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&config).await;

    let listener = TcpListener::bind("127.0.0.1:0").expect("Unable to bind random port");
    let port = listener.local_addr().unwrap().port();

    let sender = config.email_client.sender().expect("Invalid sender email");
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender,
        config.email_client.auth_token,
        std::time::Duration::from_secs(1),
    );

    let server = emailer::startup::run(listener, connection_pool.clone(), email_client)
        .expect("Failed to create server");

    let _ = tokio::spawn(server);
    (format!("http://127.0.0.1:{port}"), connection_pool)
}

pub async fn configure_database(config: &Config) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.database.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!("create database \"{}\";", config.database.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect_with(config.database.with_db())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations");

    connection_pool
}

#[actix_rt::test]
async fn health_check_test() {
    let (addr, _) = spawn_app().await;

    let client = reqwest::Client::new();

    let responce = client
        .get(format!("{addr}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(responce.status().is_success());
    assert_eq!(responce.content_length(), Some(0));
}

#[actix_rt::test]
async fn subscribe_ret_200_if_valid_form() {
    let (addr, pool) = spawn_app().await;

    let client = reqwest::Client::new();
    let body = "name=pog%20dog&email=pogolius%40gmail.com";

    let responce = client
        .post(&format!("{addr}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(responce.status().as_u16(), 200);

    let saved = sqlx::query!("select email, name from subscriptions")
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "pogolius@gmail.com");
    assert_eq!(saved.name, "pog dog");
}

#[actix_rt::test]
async fn subscribe_ret_400_if_invalid_form() {
    let (addr, _) = spawn_app().await;

    let client = reqwest::Client::new();
    let invalid_forms = vec![
        ("name=pog%20dog", "missing email"),
        ("email=pogolius%40gmail.com", "missing name"),
        ("", "missing name, and email"),
        ("name=&email=pogolius%40gmail.com", "invalid name"),
        ("name=pogdog&email=some_mail_address", "invalid email"),
        ("name=pog%20dog&email=", "invalid email"),
    ];

    for (form, error) in invalid_forms {
        let responce = client
            .post(&format!("{addr}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            responce.status().as_u16(),
            400,
            "Did not fail with invalid form, expected error: {}",
            error
        );
    }
}
