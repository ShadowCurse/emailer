use emailer::config::{read_config, Config};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

async fn spawn_app() -> (String, PgPool) {
    let mut config = read_config().expect("Failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&config).await;

    let listener = TcpListener::bind("127.0.0.1:0").expect("Unable to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server =
        emailer::startup::run(listener, connection_pool.clone()).expect("Failed to create server");
    let _ = tokio::spawn(server);
    (format!("http://127.0.0.1:{port}"), connection_pool)
}

pub async fn configure_database(config: &Config) -> PgPool {
    let mut connection = PgConnection::connect(&config.database.connection_no_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!("create database \"{}\";", config.database.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect(&config.database.connection())
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
