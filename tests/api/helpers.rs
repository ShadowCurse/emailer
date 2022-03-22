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
    pub db_pool: PgPool,
    pub email_server: MockServer,
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
