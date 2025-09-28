use crate::config::TelemetryConfig;
use std::fmt;
use tracing_subscriber::filter::ParseError;
use tracing_subscriber::EnvFilter;

#[derive(Debug)]
pub enum TelemetryError {
    EnvFilter { value: String, source: ParseError },
    Subscriber(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelemetryError::EnvFilter { value, .. } => {
                write!(
                    f,
                    "invalid log level/filter '{}': unable to build EnvFilter",
                    value
                )
            }
            TelemetryError::Subscriber(err) => write!(f, "telemetry error: {err}"),
        }
    }
}

impl std::error::Error for TelemetryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TelemetryError::EnvFilter { source, .. } => Some(source),
            TelemetryError::Subscriber(err) => Some(&**err),
        }
    }
}

pub fn init(config: &TelemetryConfig) -> Result<(), TelemetryError> {
    let env_filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => {
            EnvFilter::try_new(&config.log_level).map_err(|source| TelemetryError::EnvFilter {
                value: config.log_level.clone(),
                source,
            })?
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .with_ansi(false)
        .try_init()
        .map_err(TelemetryError::Subscriber)
}
