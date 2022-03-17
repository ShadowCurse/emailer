use emailer::{config::read_config, startup::run};
use sqlx::PgPool;
use env_logger::Env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config = read_config().expect("Failed to read config");
    let connection = config.database.connection();
    let connection_pool = PgPool::connect(&connection)
        .await
        .expect("Failed to connect to Postgres");

    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", config.application_port))
        .expect("Unable to bind port");
    run(listener, connection_pool)?.await
}
