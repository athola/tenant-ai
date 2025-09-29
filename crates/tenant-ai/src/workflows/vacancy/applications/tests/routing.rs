use super::common::*;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

use crate::workflows::vacancy::applications::domain::VacancyApplicationStatus;
use crate::workflows::vacancy::applications::repository::{
    ApplicationRecord, ApplicationRepository,
};
use crate::workflows::vacancy::applications::{
    ApplicationDecision, EvaluationOutcome, VacancyApplicationService,
};

#[tokio::test]
async fn submit_handler_returns_conflict_on_duplicate() {
    let service = Arc::new(VacancyApplicationService::new(
        Arc::new(ConflictRepository),
        Arc::new(MemoryAlerts::default()),
        evaluation_config(),
    ));

    let response = crate::workflows::vacancy::applications::router::submit_handler::<
        ConflictRepository,
        MemoryAlerts,
    >(State(service), axum::Json(submission()))
    .await;

    assert_conflict_response(response);
}

#[tokio::test]
async fn submit_handler_returns_unprocessable_for_compliance_error() {
    let service = Arc::new(VacancyApplicationService::new(
        Arc::new(MemoryRepository::default()),
        Arc::new(MemoryAlerts::default()),
        evaluation_config(),
    ));

    let response = crate::workflows::vacancy::applications::router::submit_handler::<
        MemoryRepository,
        MemoryAlerts,
    >(State(service), axum::Json(missing_income_submission()))
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn submit_handler_returns_internal_error_on_repository_failure() {
    let service = Arc::new(VacancyApplicationService::new(
        Arc::new(UnavailableRepository),
        Arc::new(MemoryAlerts::default()),
        evaluation_config(),
    ));

    let response = crate::workflows::vacancy::applications::router::submit_handler::<
        UnavailableRepository,
        MemoryAlerts,
    >(State(service), axum::Json(submission()))
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn submit_route_accepts_payloads() {
    let (service, _, _) = build_service();
    let router = application_router_with_service(service);

    let response = router
        .oneshot(
            axum::http::Request::post("/api/v1/vacancy/applications")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&submission()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("route executes");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let payload = read_json_body(response).await;
    assert!(payload.get("application_id").is_some());
}

#[tokio::test]
async fn status_handler_returns_found_records() {
    let (service, repository, alerts) = build_service();
    let service = Arc::new(service);

    let record = service.submit(submission()).expect("submission succeeds");
    repository
        .update(ApplicationRecord {
            profile: record.profile.clone(),
            status: VacancyApplicationStatus::Approved,
            evaluation: Some(EvaluationOutcome {
                application_id: record.profile.application_id.clone(),
                decision: ApplicationDecision::Approved,
                total_score: 55,
                components: Vec::new(),
            }),
        })
        .expect("update succeeds");

    let response = crate::workflows::vacancy::applications::router::status_handler::<
        MemoryRepository,
        MemoryAlerts,
    >(
        State(service.clone()),
        axum::extract::Path(record.profile.application_id.0.clone()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json_body(response).await;
    assert_eq!(
        payload
            .get("application_id")
            .and_then(serde_json::Value::as_str),
        Some(record.profile.application_id.0.as_str())
    );
    assert_eq!(
        payload.get("status").and_then(serde_json::Value::as_str),
        Some(VacancyApplicationStatus::Approved.label())
    );
    assert_eq!(
        payload
            .get("total_score")
            .and_then(serde_json::Value::as_i64),
        Some(55)
    );

    assert!(
        alerts.events().is_empty(),
        "status check should not emit alerts"
    );
}

#[tokio::test]
async fn status_handler_returns_derived_view_for_missing_record() {
    let (service, repository, alerts) = build_service();
    let service = Arc::new(service);

    let record = service.submit(submission()).expect("submission succeeds");

    let response = crate::workflows::vacancy::applications::router::status_handler::<
        MemoryRepository,
        MemoryAlerts,
    >(
        State(service),
        axum::extract::Path(format!("{}-missing", record.profile.application_id.0)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json_body(response).await;
    assert_eq!(payload.get("status"), Some(&json!("submitted")));
    assert!(matches!(
        payload.get("total_score"),
        None | Some(Value::Null)
    ));
    assert!(payload
        .get("decision_rationale")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .contains("pending"));

    assert!(repository.pending(10).unwrap().is_empty());
    assert!(alerts.events().is_empty());
}
