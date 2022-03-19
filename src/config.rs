use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub application: AppConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseConfig {
    pub fn connection(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
    pub fn connection_no_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

#[derive(Deserialize)]
pub struct AppConfig {
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
        .add_source(config::File::from(config_dir.join(Into::<&str>::into(env))).required(true));
    builder.build()?.try_deserialize()
}
