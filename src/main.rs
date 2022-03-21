use emailer::{config::read_config, startup::AppServer, telemetry::init_logging};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logging("emailer", "info", std::io::stdout);

    let config = read_config().expect("Failed to read config");
    let server = AppServer::build(config).await?;
    server.run().await?;
    Ok(())
}
