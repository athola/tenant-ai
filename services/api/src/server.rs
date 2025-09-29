use crate::cli::ServeArgs;
use crate::infra::{
    default_evaluation_config, AppState, InMemoryAlertPublisher, InMemoryApplicationRepository,
};
use crate::routes::with_application_routes;
use axum::Extension;
use axum_prometheus::PrometheusMetricLayer;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tenant_ai::config::AppConfig;
use tenant_ai::error::AppError;
use tenant_ai::telemetry;
use tenant_ai::workflows::vacancy::applications::VacancyApplicationService;
use tracing::info;

pub(crate) async fn run(mut args: ServeArgs) -> Result<(), AppError> {
    let mut config = AppConfig::load()?;

    if let Some(host) = args.host.take() {
        config.server.host = host;
    }
    if let Some(port) = args.port.take() {
        config.server.port = port;
    }

    telemetry::init(&config.telemetry)?;

    let (prometheus_layer, prometheus_handle) = PrometheusMetricLayer::pair();
    let readiness_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let app_state = AppState {
        readiness: readiness_flag.clone(),
        metrics: Arc::new(prometheus_handle),
    };

    let repository = Arc::new(InMemoryApplicationRepository::default());
    let alerts = Arc::new(InMemoryAlertPublisher::default());
    let evaluation_config = default_evaluation_config();
    let application_service = Arc::new(VacancyApplicationService::new(
        repository,
        alerts,
        evaluation_config,
    ));

    let app = with_application_routes(application_service)
        .layer(Extension(app_state))
        .layer(prometheus_layer);

    let addr = config.server.socket_addr()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    readiness_flag.store(true, Ordering::Release);

    info!(?config.environment, %addr, "agentic workflow orchestrator ready");

    axum::serve(listener, app).await?;
    Ok(())
}
