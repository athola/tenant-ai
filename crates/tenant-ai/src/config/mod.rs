use std::env;
use std::fmt;
use std::net::{IpAddr, SocketAddr};

/// Distinguishes runtime behavior for different stages of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Test,
    Production,
}

impl AppEnvironment {
    fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "prod" | "production" => Self::Production,
            "test" | "ci" => Self::Test,
            _ => Self::Development,
        }
    }
}

/// Top-level configuration for the application.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub environment: AppEnvironment,
    pub server: ServerConfig,
    pub telemetry: TelemetryConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let environment = AppEnvironment::from_str(
            &env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
        );

        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort)?;

        let log_level = env::var("APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            environment,
            server: ServerConfig { host, port },
            telemetry: TelemetryConfig { log_level },
        })
    }
}

/// Settings controlling the HTTP server binding.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        if self.host.eq_ignore_ascii_case("localhost") {
            return Ok(SocketAddr::new(IpAddr::from([127, 0, 0, 1]), self.port));
        }

        let ip: IpAddr = self
            .host
            .parse()
            .map_err(|source| ConfigError::InvalidHost { source })?;

        Ok(SocketAddr::new(ip, self.port))
    }
}

/// Tracing and metrics controls.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub log_level: String,
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPort,
    InvalidHost { source: std::net::AddrParseError },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidPort => write!(f, "APP_PORT must be a valid u16"),
            ConfigError::InvalidHost { .. } => {
                write!(f, "APP_HOST must parse to an IPv4 or IPv6 address")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::InvalidPort => None,
            ConfigError::InvalidHost { source } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> &'static Mutex<()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD.get_or_init(|| Mutex::new(()))
    }

    fn reset_env() {
        env::remove_var("APP_ENV");
        env::remove_var("APP_HOST");
        env::remove_var("APP_PORT");
        env::remove_var("APP_LOG_LEVEL");
    }

    #[test]
    fn load_uses_defaults_when_env_missing() {
        let _lock = env_guard().lock().expect("env mutex poisoned");
        reset_env();
        let config = AppConfig::load().expect("config loads with defaults");
        assert_eq!(config.environment, AppEnvironment::Development);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.telemetry.log_level, "info");
    }

    #[test]
    fn accepts_localhost_host() {
        let _lock = env_guard().lock().expect("env mutex poisoned");
        reset_env();
        env::set_var("APP_HOST", "localhost");
        let config = AppConfig::load().expect("config loads");
        let addr = config.server.socket_addr().expect("localhost resolves");
        assert_eq!(addr, SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 3000));
    }
}
