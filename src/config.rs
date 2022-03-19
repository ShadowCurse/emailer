use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgSslMode, PgConnectOptions};

#[derive(Deserialize)]
pub struct Config {
    pub application: AppConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseConfig {
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password)
            .port(self.port)
            .ssl_mode(ssl_mode)
    }
}

#[derive(Deserialize)]
pub struct AppConfig {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

impl AppConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub enum Environment {
    Local,
    Production,
}

impl From<Environment> for &'static str {
    fn from(env: Environment) -> Self {
        match env {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            s => Err(format!(
                "{} is not supported environment. Use either `local` or `production`",
                s
            )),
        }
    }
}

pub fn read_config() -> Result<Config, config::ConfigError> {
    let curr_dir = std::env::current_dir().expect("Failed to determine the current directory");
    let config_dir = curr_dir.join("config");

    let env: Environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| Into::<&str>::into(Environment::Local).to_string())
        .try_into()
        .expect("Failed to parse APP_ENV");

    let builder = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")).required(true))
        .add_source(config::File::from(config_dir.join(Into::<&str>::into(env))).required(true))
        .add_source(config::Environment::with_prefix("app").separator("__"));
    builder.build()?.try_deserialize()
}
