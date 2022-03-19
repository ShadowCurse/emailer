use emailer::{config::read_config, startup::run, telemetry::init_logging};
use sqlx::PgPool;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logging("emailer", "info", std::io::stdout);

    let config = read_config().expect("Failed to read config");
    let connection = config.database.with_db();
    let connection_pool = PgPool::connect_with(connection)
        .await
        .expect("Failed to connect to Postgres");

    let listener =
        std::net::TcpListener::bind(config.application.address()).expect("Unable to bind port");
    run(listener, connection_pool)?.await
}
