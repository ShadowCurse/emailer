use crate::config::Config;
use crate::{email_client::EmailClient, routes::*};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct AppServer {
    port: u16,
    server: Server,
}

#[derive(Debug)]
pub struct AppBaseUrl(pub String);

impl AppServer {
    pub async fn build(config: Config) -> Result<Self, std::io::Error> {
        let connection = config.database.with_db();
        let connection_pool = PgPool::connect_with(connection)
            .await
            .expect("Failed to connect to Postgres");
        let sender = config.email_client.sender().expect("Invalid sender email");
        let email_client = EmailClient::new(
            config.email_client.base_url,
            sender,
            config.email_client.auth_token,
            std::time::Duration::from_secs(5),
        );
        let listener =
            TcpListener::bind(config.application.address()).expect("Unable to bind port");
        let port = listener.local_addr().unwrap().port();
        let server = Self::running_server(
            listener,
            connection_pool,
            email_client,
            config.application.base_url,
        )?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    fn running_server(
        listener: TcpListener,
        connection_pool: PgPool,
        email_client: EmailClient,
        base_url: String,
    ) -> Result<Server, std::io::Error> {
        let connection_pool = web::Data::new(connection_pool);
        let email_client = web::Data::new(email_client);
        let base_url = web::Data::new(AppBaseUrl(base_url));
        let server = HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .route("/health_check", web::get().to(health_check))
                .route("/subscriptions", web::post().to(subscribe))
                .route("/subscriptions/confirm", web::get().to(confirm))
                .app_data(connection_pool.clone())
                .app_data(email_client.clone())
                .app_data(base_url.clone())
        })
        .listen(listener)?
        .run();
        Ok(server)
    }
}
