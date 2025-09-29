use crate::infra::{deserialize_date, deserialize_optional_date, AppState};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Cursor;
use std::sync::Arc;
use tenant_ai::error::AppError;
use tenant_ai::workflows::apollo::ApolloVacancyImporter;
use tenant_ai::workflows::vacancy::applications::{
    application_router, AlertPublisher, ApplicationRepository, VacancyApplicationService,
};
use tenant_ai::workflows::vacancy::{
    report::views::{
        ComplianceAlertView, RoleLoadEntry, StageProgressEntry, TaskSnapshotView, VacancyInsights,
    },
    TaskDetailView, VacancyWorkflowBlueprint, VacancyWorkflowInstance,
};

#[derive(Debug, Deserialize)]
pub(crate) struct VacancyReportRequest {
    #[serde(deserialize_with = "deserialize_date")]
    pub(crate) vacancy_start: NaiveDate,
    #[serde(deserialize_with = "deserialize_date")]
    pub(crate) target_move_in: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    pub(crate) today: Option<NaiveDate>,
    #[serde(default)]
    pub(crate) include_tasks: bool,
    #[serde(default)]
    pub(crate) apollo_csv: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VacancyReportResponse {
    pub(crate) vacancy_start: NaiveDate,
    pub(crate) target_move_in: NaiveDate,
    pub(crate) today: NaiveDate,
    pub(crate) data_source: VacancyDataSource,
    pub(crate) stage_progress: Vec<StageProgressEntry>,
    pub(crate) role_load: Vec<RoleLoadEntry>,
    pub(crate) overdue_tasks: Vec<TaskSnapshotView>,
    pub(crate) compliance_alerts: Vec<ComplianceAlertView>,
    pub(crate) insights: VacancyInsights,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tasks: Option<Vec<TaskDetailView>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VacancyDataSource {
    Apollo,
    Standard,
}

pub(crate) fn with_application_routes<R, A>(
    service: Arc<VacancyApplicationService<R, A>>,
) -> axum::Router
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    application_router(service)
        .route("/health", axum::routing::get(healthcheck))
        .route("/ready", axum::routing::get(readiness_endpoint))
        .route("/metrics", axum::routing::get(metrics_endpoint))
        .route(
            "/api/v1/vacancy/report",
            axum::routing::post(vacancy_report_endpoint),
        )
}

pub(crate) async fn healthcheck() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

pub(crate) async fn readiness_endpoint(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ready = state.readiness.load(std::sync::atomic::Ordering::Relaxed);
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let payload = if ready {
        json!({ "status": "ready" })
    } else {
        json!({ "status": "initializing" })
    };

    (status, Json(payload))
}

pub(crate) async fn metrics_endpoint(Extension(state): Extension<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
}

pub(crate) async fn vacancy_report_endpoint(
    Json(payload): Json<VacancyReportRequest>,
) -> Result<Json<VacancyReportResponse>, AppError> {
    let VacancyReportRequest {
        vacancy_start,
        target_move_in,
        today,
        include_tasks,
        apollo_csv,
    } = payload;

    let (instance, data_source) = if let Some(csv) = apollo_csv {
        let reader = Cursor::new(csv.into_bytes());
        let instance = ApolloVacancyImporter::from_reader(reader, vacancy_start, target_move_in)?;
        (instance, VacancyDataSource::Apollo)
    } else {
        let blueprint = VacancyWorkflowBlueprint::standard();
        let instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);
        (instance, VacancyDataSource::Standard)
    };

    let today = today.unwrap_or_else(|| Local::now().date_naive());
    let report = instance.report(today);
    let summary = report.summary();
    let insights = summary.insights(&instance, vacancy_start, target_move_in, today);
    let tasks = if include_tasks {
        Some(instance.task_details())
    } else {
        None
    };

    Ok(Json(VacancyReportResponse {
        vacancy_start,
        target_move_in,
        today,
        data_source,
        stage_progress: summary.stage_progress,
        role_load: summary.role_load,
        overdue_tasks: summary.overdue_tasks,
        compliance_alerts: summary.compliance_alerts,
        insights,
        tasks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    fn sample_dates() -> (NaiveDate, NaiveDate) {
        let vacancy_start = NaiveDate::from_ymd_opt(2025, 9, 24).expect("valid start date");
        let target_move_in = vacancy_start
            .checked_add_signed(chrono::Duration::days(14))
            .expect("valid move-in date");
        (vacancy_start, target_move_in)
    }

    #[tokio::test]
    async fn vacancy_report_endpoint_returns_summary() {
        let (vacancy_start, target_move_in) = sample_dates();
        let request = VacancyReportRequest {
            vacancy_start,
            target_move_in,
            today: None,
            include_tasks: false,
            apollo_csv: None,
        };

        let Json(body) = vacancy_report_endpoint(Json(request))
            .await
            .expect("report builds");

        assert_eq!(body.data_source, VacancyDataSource::Standard);
        assert_eq!(body.stage_progress.len(), 4);
        assert!(body.tasks.is_none());
        assert!(body.insights.readiness_score <= 100);
    }

    #[tokio::test]
    async fn vacancy_report_endpoint_can_include_tasks() {
        let (vacancy_start, target_move_in) = sample_dates();
        let request = VacancyReportRequest {
            vacancy_start,
            target_move_in,
            today: None,
            include_tasks: true,
            apollo_csv: Some(
                "Task ID,Created At,Completed At,Last Modified,Name\n1,2025-09-24T10:00:00Z,2025-09-25T12:15:00Z,2025-09-25T12:15:00Z,Create and Publish Listing - Leasing Agent\n".to_string(),
            ),
        };

        let Json(body) = vacancy_report_endpoint(Json(request))
            .await
            .expect("report builds");

        assert_eq!(body.data_source, VacancyDataSource::Apollo);
        let tasks = body.tasks.expect("tasks returned");
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].status_label, "Completed");
        assert!(body.insights.focus_stage.is_some());
    }
}
