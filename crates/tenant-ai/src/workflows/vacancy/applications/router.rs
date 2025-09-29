use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde_json::json;

use super::domain::{ApplicationId, ApplicationSubmission, VacancyApplicationStatus};
use super::repository::{AlertPublisher, ApplicationRepository, RepositoryError};
use super::service::{ApplicationServiceError, VacancyApplicationService};

/// Router builder exposing HTTP endpoints for intake and evaluation.
pub fn application_router<R, A>(service: Arc<VacancyApplicationService<R, A>>) -> Router
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    Router::new()
        .route("/api/v1/vacancy/applications", post(submit_handler::<R, A>))
        .route(
            "/api/v1/vacancy/applications/:application_id",
            get(status_handler::<R, A>),
        )
        .with_state(service)
}

pub(crate) async fn submit_handler<R, A>(
    State(service): State<Arc<VacancyApplicationService<R, A>>>,
    axum::Json(submission): axum::Json<ApplicationSubmission>,
) -> Response
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    match service.submit(submission) {
        Ok(record) => {
            let view = record.status_view();
            (StatusCode::ACCEPTED, axum::Json(view)).into_response()
        }
        Err(ApplicationServiceError::Compliance(error)) => {
            let payload = json!({
                "error": error.to_string(),
            });
            (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(payload)).into_response()
        }
        Err(ApplicationServiceError::Repository(RepositoryError::Conflict)) => {
            let payload = json!({
                "error": "application already exists",
            });
            (StatusCode::CONFLICT, axum::Json(payload)).into_response()
        }
        Err(other) => {
            let payload = json!({
                "error": other.to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(payload)).into_response()
        }
    }
}

pub(crate) async fn status_handler<R, A>(
    State(service): State<Arc<VacancyApplicationService<R, A>>>,
    Path(application_id): Path<String>,
) -> Response
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    let id = ApplicationId(application_id);
    match service.get(&id) {
        Ok(record) => {
            let view = record.status_view();
            (StatusCode::OK, axum::Json(view)).into_response()
        }
        Err(ApplicationServiceError::Repository(RepositoryError::NotFound)) => {
            let payload = json!({
                "application_id": id.0,
                "status": VacancyApplicationStatus::Submitted.label(),
                "decision_rationale": "pending evaluation",
                "total_score": serde_json::Value::Null,
            });
            (StatusCode::OK, axum::Json(payload)).into_response()
        }
        Err(other) => {
            let payload = json!({
                "error": other.to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(payload)).into_response()
        }
    }
}
