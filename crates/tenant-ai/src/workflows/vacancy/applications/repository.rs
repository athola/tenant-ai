use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::domain::{ApplicantProfile, ApplicationId, VacancyApplicationStatus};
use super::evaluation::EvaluationOutcome;

/// Repository record containing the profile, evaluation, and status metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRecord {
    pub profile: ApplicantProfile,
    pub status: VacancyApplicationStatus,
    pub evaluation: Option<EvaluationOutcome>,
}

impl ApplicationRecord {
    pub fn decision_rationale(&self) -> String {
        match &self.evaluation {
            Some(outcome) => outcome.decision.summary(),
            None => "pending evaluation".to_string(),
        }
    }

    pub fn status_view(&self) -> ApplicationStatusView {
        ApplicationStatusView {
            application_id: self.profile.application_id.clone(),
            status: self.status.label(),
            decision_rationale: self.decision_rationale(),
            total_score: self.evaluation.as_ref().map(|outcome| outcome.total_score),
        }
    }
}

/// Storage abstraction so the service module can be exercised in isolation.
pub trait ApplicationRepository: Send + Sync {
    fn insert(&self, record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError>;
    fn update(&self, record: ApplicationRecord) -> Result<(), RepositoryError>;
    fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError>;
    fn pending(&self, limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError>;
}

/// Error enumeration for repository failures.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("record already exists")]
    Conflict,
    #[error("record not found")]
    NotFound,
    #[error("repository unavailable: {0}")]
    Unavailable(String),
}

/// Trait describing outbound alert hooks (e.g., AppFolio or e-mail adapters).
pub trait AlertPublisher: Send + Sync {
    fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError>;
}

/// Simple alert payload so routes/tests can assert integration boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppFolioAlert {
    pub template: String,
    pub application_id: ApplicationId,
    pub details: BTreeMap<String, String>,
}

/// Alert dispatch error.
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("alert transport unavailable: {0}")]
    Transport(String),
}

/// Sanitized representation of an application's exposed status.
#[derive(Debug, Clone, Serialize)]
pub struct ApplicationStatusView {
    pub application_id: ApplicationId,
    pub status: &'static str,
    pub decision_rationale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_score: Option<i16>,
}
