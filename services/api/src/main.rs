use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Extension;
use axum::Json;
use axum_prometheus::PrometheusMetricLayer;
use chrono::{Local, NaiveDate};
use clap::{Args, Parser, Subcommand};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tenant_ai::config::AppConfig;
use tenant_ai::error::AppError;
use tenant_ai::telemetry;
use tenant_ai::workflows::apollo::ApolloVacancyImporter;
use tenant_ai::workflows::vacancy::applications::{
    application_router, AlertError, AlertPublisher, AppFolioAlert, ApplicationId,
    ApplicationRecord, ApplicationRepository, ComplianceGuard, EvaluationConfig, RepositoryError,
    VacancyApplicationService, VacancyApplicationStatus,
};
use tenant_ai::workflows::vacancy::{
    ComplianceAlertView, RoleLoadEntry, StageProgressEntry, TaskDetailView, TaskSnapshotView,
    VacancyReport, VacancyWorkflowBlueprint, VacancyWorkflowInstance,
};
use tracing::info;

#[derive(Clone)]
struct AppState {
    readiness: Arc<AtomicBool>,
    metrics: Arc<PrometheusHandle>,
}

#[derive(Default, Clone)]
struct InMemoryApplicationRepository {
    records: Arc<Mutex<HashMap<ApplicationId, ApplicationRecord>>>,
}

impl ApplicationRepository for InMemoryApplicationRepository {
    fn insert(&self, record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError> {
        let mut guard = self.records.lock().expect("repository mutex poisoned");
        if guard.contains_key(&record.profile.application_id) {
            return Err(RepositoryError::Conflict);
        }
        guard.insert(record.profile.application_id.clone(), record.clone());
        Ok(record)
    }

    fn update(&self, record: ApplicationRecord) -> Result<(), RepositoryError> {
        let mut guard = self.records.lock().expect("repository mutex poisoned");
        if guard.contains_key(&record.profile.application_id) {
            guard.insert(record.profile.application_id.clone(), record);
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("repository mutex poisoned");
        Ok(guard.get(id).cloned())
    }

    fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("repository mutex poisoned");
        Ok(guard
            .values()
            .filter(|record| record.status == VacancyApplicationStatus::UnderReview)
            .cloned()
            .collect())
    }
}

#[derive(Default, Clone)]
struct InMemoryAlertPublisher {
    events: Arc<Mutex<Vec<AppFolioAlert>>>,
}

impl AlertPublisher for InMemoryAlertPublisher {
    fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError> {
        let mut guard = self.events.lock().expect("alert mutex poisoned");
        guard.push(alert);
        Ok(())
    }
}

fn default_evaluation_config() -> EvaluationConfig {
    EvaluationConfig {
        minimum_rent_to_income_ratio: 0.28,
        minimum_credit_score: Some(650),
        max_evictions: 0,
        violent_felony_lookback_years: 7,
        non_violent_lookback_years: 5,
        misdemeanor_lookback_years: 3,
        deposit_cap_multiplier: 2.0,
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "Agentic Property Orchestrator",
    about = "Demonstrate and run the Agentic Property Orchestrator from the command line",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the HTTP service (default command)
    Serve(ServeArgs),
    /// Generate a vacancy readiness report for stakeholder demos
    Vacancy {
        #[command(subcommand)]
        command: VacancyCommand,
    },
}

#[derive(Args, Debug, Default)]
struct ServeArgs {
    /// Override the configured host for the HTTP server
    #[arg(long)]
    host: Option<String>,
    /// Override the configured port for the HTTP server
    #[arg(long)]
    port: Option<u16>,
}

#[derive(Subcommand, Debug)]
enum VacancyCommand {
    /// Generate a vacancy workflow report and optional task listing
    Report(VacancyReportArgs),
}

#[derive(Args, Debug)]
struct VacancyReportArgs {
    /// Vacancy start date (YYYY-MM-DD)
    #[arg(long, value_parser = parse_date)]
    vacancy_start: NaiveDate,
    /// Target move-in date (YYYY-MM-DD)
    #[arg(long, value_parser = parse_date)]
    target_move_in: NaiveDate,
    /// Evaluation date for the report (defaults to today)
    #[arg(long, value_parser = parse_date)]
    today: Option<NaiveDate>,
    /// Optional Apollo CSV export to hydrate task progress
    #[arg(long)]
    apollo_csv: Option<PathBuf>,
    /// Include a full task listing in the output
    #[arg(long)]
    list_tasks: bool,
}

#[derive(Debug, Deserialize)]
struct VacancyReportRequest {
    #[serde(deserialize_with = "deserialize_date")]
    vacancy_start: NaiveDate,
    #[serde(deserialize_with = "deserialize_date")]
    target_move_in: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    today: Option<NaiveDate>,
    #[serde(default)]
    include_tasks: bool,
    #[serde(default)]
    apollo_csv: Option<String>,
}

#[derive(Debug, Serialize)]
struct VacancyReportResponse {
    vacancy_start: NaiveDate,
    target_move_in: NaiveDate,
    today: NaiveDate,
    data_source: VacancyDataSource,
    stage_progress: Vec<StageProgressEntry>,
    role_load: Vec<RoleLoadEntry>,
    overdue_tasks: Vec<TaskSnapshotView>,
    compliance_alerts: Vec<ComplianceAlertView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tasks: Option<Vec<TaskDetailView>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum VacancyDataSource {
    Apollo,
    Standard,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run_cli().await {
        eprintln!("application error: {err}");
        std::process::exit(1);
    }
}

async fn run_cli() -> Result<(), AppError> {
    let cli = Cli::parse();
    let command = cli
        .command
        .unwrap_or_else(|| Command::Serve(ServeArgs::default()));

    match command {
        Command::Serve(args) => run_server(args).await,
        Command::Vacancy {
            command: VacancyCommand::Report(args),
        } => run_vacancy_report(args),
    }
}

fn parse_date(raw: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .map_err(|err| format!("failed to parse '{raw}' as YYYY-MM-DD ({err})"))
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_date(&raw).map_err(serde::de::Error::custom)
}

fn deserialize_optional_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    opt.map(|value| parse_date(&value).map_err(serde::de::Error::custom))
        .transpose()
}

async fn run_server(mut args: ServeArgs) -> Result<(), AppError> {
    let mut config = AppConfig::load()?;

    if let Some(host) = args.host.take() {
        config.server.host = host;
    }
    if let Some(port) = args.port.take() {
        config.server.port = port;
    }

    telemetry::init(&config.telemetry)?;

    let (prometheus_layer, prometheus_handle) = PrometheusMetricLayer::pair();
    let readiness_flag = Arc::new(AtomicBool::new(false));
    let app_state = AppState {
        readiness: readiness_flag.clone(),
        metrics: Arc::new(prometheus_handle),
    };

    let compliance_guard = ComplianceGuard::new();
    let repository = Arc::new(InMemoryApplicationRepository::default());
    let alerts = Arc::new(InMemoryAlertPublisher::default());
    let evaluation_config = default_evaluation_config();
    let application_service = Arc::new(VacancyApplicationService::new(
        compliance_guard,
        repository,
        alerts,
        evaluation_config,
    ));

    let app = application_router(application_service.clone())
        .route("/health", get(healthcheck))
        .route("/ready", get(readiness_endpoint))
        .route("/metrics", get(metrics_endpoint))
        .route("/api/v1/vacancy/report", post(vacancy_report_endpoint))
        .layer(Extension(app_state))
        .layer(prometheus_layer);

    let addr = config.server.socket_addr()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    readiness_flag.store(true, Ordering::Release);

    info!(?config.environment, %addr, "agentic workflow orchestrator ready");

    axum::serve(listener, app).await?;
    Ok(())
}

fn run_vacancy_report(args: VacancyReportArgs) -> Result<(), AppError> {
    let VacancyReportArgs {
        vacancy_start,
        target_move_in,
        today,
        apollo_csv,
        list_tasks,
    } = args;

    let today = today.unwrap_or_else(|| Local::now().date_naive());
    let imported = apollo_csv.is_some();

    let instance = match apollo_csv {
        Some(path) => ApolloVacancyImporter::from_path(path, vacancy_start, target_move_in)?,
        None => {
            let blueprint = VacancyWorkflowBlueprint::standard();
            VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in)
        }
    };

    let report = instance.report(today);
    render_vacancy_report(
        &instance,
        &report,
        vacancy_start,
        target_move_in,
        today,
        imported,
        list_tasks,
    );

    Ok(())
}

async fn healthcheck() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn readiness_endpoint(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ready = state.readiness.load(Ordering::Relaxed);
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

async fn metrics_endpoint(Extension(state): Extension<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
}

async fn vacancy_report_endpoint(
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
        tasks,
    }))
}

fn render_vacancy_report(
    instance: &VacancyWorkflowInstance,
    report: &VacancyReport,
    vacancy_start: NaiveDate,
    target_move_in: NaiveDate,
    today: NaiveDate,
    imported: bool,
    list_tasks: bool,
) {
    println!("Vacancy workflow demo");
    println!(
        "Vacancy window: {} -> {} (evaluated {})",
        vacancy_start, target_move_in, today
    );

    if imported {
        println!("Data source: Apollo CSV import");
    } else {
        println!("Data source: Standard blueprint (no Apollo data provided)");
    }

    let summary = report.summary();

    println!("\nStage progress");
    for progress in &summary.stage_progress {
        println!(
            "- {}: {}/{} tasks completed",
            progress.stage_label, progress.completed, progress.total
        );
    }

    println!("\nRole workload");
    for load in &summary.role_load {
        println!(
            "- {}: {} open, {} overdue",
            load.role_label, load.open, load.overdue
        );
    }

    if summary.overdue_tasks.is_empty() {
        println!("\nOverdue tasks: none");
    } else {
        println!("\nOverdue tasks");
        for task in &summary.overdue_tasks {
            println!(
                "- {} ({}), role {}, due {}, status {}",
                task.name, task.stage_label, task.role_label, task.due_date, task.status_label
            );
        }
    }

    if summary.compliance_alerts.is_empty() {
        println!("\nCompliance alerts: none");
    } else {
        println!("\nCompliance alerts");
        for alert in &summary.compliance_alerts {
            println!(
                "- [{}] {}: {}",
                alert.severity_label, alert.topic, alert.detail
            );
        }
    }

    if list_tasks {
        println!("\nTask breakdown by due date");
        for task in instance.task_details() {
            let completion_note = match task.completed_on {
                Some(date) => format!(" (completed {date})"),
                None => String::new(),
            };
            println!(
                "- {} | {} | {} | due {} | status {}{}",
                task.key,
                task.name,
                task.stage_label,
                task.due_date,
                task.status_label,
                completion_note
            );
        }
    }
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

        let Json(body) = super::vacancy_report_endpoint(Json(request))
            .await
            .expect("report builds");

        assert_eq!(body.data_source, VacancyDataSource::Standard);
        assert_eq!(body.stage_progress.len(), 4);
        assert!(body.tasks.is_none());
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

        let Json(body) = super::vacancy_report_endpoint(Json(request))
            .await
            .expect("report builds");

        assert_eq!(body.data_source, VacancyDataSource::Apollo);
        let tasks = body.tasks.expect("tasks returned");
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].status_label, "Completed");
    }
}
