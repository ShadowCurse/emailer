use emailer::config::{read_config, Config};
use emailer::startup::AppServer;
use emailer::telemetry::init_logging;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_logging("test", "debug", std::io::stdout);
    } else {
        init_logging("test", "debug", std::io::sink);
    }
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct Links {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subsciptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub fn get_links(&self, request: &wiremock::Request) -> Links {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        let get_link = |s| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| l.kind() == &linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let link = links[0].as_str();
            let mut link = reqwest::Url::parse(link).unwrap();
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");
            link.set_port(Some(self.port)).unwrap();
            link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let text = get_link(body["TextBody"].as_str().unwrap());
        Links {
            html,
            text,
        }
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let mut config = read_config().expect("Failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();
    config.application.port = 0;
    config.email_client.base_url = email_server.uri();

    let db_pool = configure_database(&config).await;

    let server = AppServer::build(config).await.unwrap();
    let port = server.port();

    let _ = tokio::spawn(server.run());

    TestApp {
        address: format!("http://127.0.0.1:{port}"),
        port,
        db_pool,
        email_server,
    }
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
