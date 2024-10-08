use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub ai_service: AIServiceSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AIServiceSettings {
    pub url: String,
    pub api_key: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::default();

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name("config/default.toml"))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional_
        let env = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false))?;

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("app"))?;

        // Deserialize the entire configuration
        s.try_into()
    }
}

impl TryFrom<Config> for Settings {
    type Error = ConfigError;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        Ok(Settings {
            server: config.get("server")?,
            ai_service: config.get("ai_service")?,
        })
    }
}