use emailer::config::{read_config, Config};
use emailer::telemetry::init_logging;
use emailer::startup::AppServer;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_logging("test", "debug", std::io::stdout);
    } else {
        init_logging("test", "debug", std::io::sink);
    }
});

pub async fn spawn_app() -> (String, PgPool) {
    Lazy::force(&TRACING);

    let mut config = read_config().expect("Failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();
    config.application.port = 0;

    let connection_pool = configure_database(&config).await;

    let server = AppServer::build(config).await.unwrap();
    let port = server.port();

    let _ = tokio::spawn(server.run());
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
