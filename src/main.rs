use emailer::{config::read_config, startup::run, telemetry::init_logging};
use sqlx::PgPool;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logging("emailer", "info", std::io::stdout);

    let config = read_config().expect("Failed to read config");
    let connection = config.database.connection();
    let connection_pool = PgPool::connect(&connection)
        .await
        .expect("Failed to connect to Postgres");

    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", config.application_port))
        .expect("Unable to bind port");
    run(listener, connection_pool)?.await
}
