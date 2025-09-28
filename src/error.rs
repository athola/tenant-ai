use crate::config::ConfigError;
use crate::telemetry::TelemetryError;
use crate::workflows::apollo::ApolloVacancyImportError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Telemetry(TelemetryError),
    Io(std::io::Error),
    Server(axum::Error),
    Workflow(ApolloVacancyImportError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(err) => write!(f, "configuration error: {}", err),
            AppError::Telemetry(err) => write!(f, "telemetry error: {}", err),
            AppError::Io(err) => write!(f, "io error: {}", err),
            AppError::Server(err) => write!(f, "server error: {}", err),
            AppError::Workflow(err) => write!(f, "workflow error: {}", err),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Config(err) => Some(err),
            AppError::Telemetry(err) => Some(err),
            AppError::Io(err) => Some(err),
            AppError::Server(err) => Some(err),
            AppError::Workflow(err) => Some(err),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Workflow(_) => StatusCode::BAD_REQUEST,
            AppError::Config(_)
            | AppError::Telemetry(_)
            | AppError::Io(_)
            | AppError::Server(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

impl From<ConfigError> for AppError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<TelemetryError> for AppError {
    fn from(value: TelemetryError) -> Self {
        Self::Telemetry(value)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<axum::Error> for AppError {
    fn from(value: axum::Error) -> Self {
        Self::Server(value)
    }
}

impl From<ApolloVacancyImportError> for AppError {
    fn from(value: ApolloVacancyImportError) -> Self {
        Self::Workflow(value)
    }
}
