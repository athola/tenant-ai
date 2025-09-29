use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::compliance::{ComplianceGuard, ComplianceViolation};
use super::domain::{ApplicationId, ApplicationSubmission, VacancyApplicationStatus};
use super::evaluation::{
    ApplicationDecision, EvaluationConfig, EvaluationEngine, EvaluationOutcome,
};
use super::repository::{
    AlertError, AlertPublisher, AppFolioAlert, ApplicationRecord, ApplicationRepository,
    RepositoryError,
};

/// Service composing the compliance guard, repository, and evaluation rubric.
pub struct VacancyApplicationService<R, A> {
    guard: Arc<ComplianceGuard>,
    repository: Arc<R>,
    alerts: Arc<A>,
    engine: Arc<EvaluationEngine>,
}

static APPLICATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn next_application_id() -> ApplicationId {
    let id = APPLICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    ApplicationId(format!("app-{id:06}"))
}

impl<R, A> VacancyApplicationService<R, A>
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    pub fn new(repository: Arc<R>, alerts: Arc<A>, config: EvaluationConfig) -> Self {
        let guard = ComplianceGuard::from_config(&config);
        Self::with_guard(guard, repository, alerts, config)
    }

    pub(crate) fn with_guard(
        guard: ComplianceGuard,
        repository: Arc<R>,
        alerts: Arc<A>,
        config: EvaluationConfig,
    ) -> Self {
        let guard = if (guard.policy().deposit_cap_multiplier() - config.deposit_cap_multiplier)
            .abs()
            < f32::EPSILON
        {
            guard
        } else {
            ComplianceGuard::from_config(&config)
        };

        let guard = Arc::new(guard);
        let engine = Arc::new(EvaluationEngine::new(config));

        Self {
            guard,
            repository,
            alerts,
            engine,
        }
    }

    /// Submit a new application, returning the repository-backed record.
    pub fn submit(
        &self,
        submission: ApplicationSubmission,
    ) -> Result<ApplicationRecord, ApplicationServiceError> {
        let mut profile = self.guard.profile_from_submission(submission)?;
        let application_id = next_application_id();
        profile.application_id = application_id.clone();

        let record = ApplicationRecord {
            profile,
            status: VacancyApplicationStatus::Submitted,
            evaluation: None,
        };

        let stored = self.repository.insert(record)?;
        Ok(stored)
    }

    /// Evaluate a pending application and persist the outcome.
    pub fn evaluate(
        &self,
        application_id: &ApplicationId,
    ) -> Result<EvaluationOutcome, ApplicationServiceError> {
        let mut record = self
            .repository
            .fetch(application_id)?
            .ok_or(RepositoryError::NotFound)?;

        let outcome = self.engine.score(&record.profile);

        record.status = match outcome.decision {
            ApplicationDecision::Approved => VacancyApplicationStatus::Approved,
            ApplicationDecision::Denied(_) => VacancyApplicationStatus::Denied,
            _ => VacancyApplicationStatus::UnderReview,
        };
        record.evaluation = Some(outcome.clone());

        self.repository.update(record)?;

        if matches!(outcome.decision, ApplicationDecision::Approved) {
            let mut details = BTreeMap::new();
            details.insert("decision".to_string(), "approved".to_string());
            self.alerts.publish(AppFolioAlert {
                template: "applicant_approved".to_string(),
                application_id: outcome.application_id.clone(),
                details,
            })?;
        }

        Ok(outcome)
    }

    /// Fetch an application and current status for API responses.
    pub fn get(
        &self,
        application_id: &ApplicationId,
    ) -> Result<ApplicationRecord, ApplicationServiceError> {
        let record = self
            .repository
            .fetch(application_id)?
            .ok_or(RepositoryError::NotFound)?;
        Ok(record)
    }
}

/// Error raised by the application service.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationServiceError {
    #[error(transparent)]
    Compliance(#[from] ComplianceViolation),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Alert(#[from] AlertError),
}
