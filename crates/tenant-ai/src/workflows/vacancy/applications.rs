//! Vacancy application intake, evaluation, and compliance scaffolding.
//!
//! The concrete implementations are intentionally left as `todo!()` placeholders so that
//! the accompanying tests can drive out the full behavior using a TDD workflow. The types
//! and signatures defined here represent the initial contract that the new vacancy intake
//! pipeline will satisfy once implemented.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Identifier wrapper for submitted applications.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationId(pub String);

/// Minimal description of the advertised vacancy used during intake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacancyListingSnapshot {
    pub unit_id: String,
    pub property_code: String,
    pub listed_rent: u32,
    pub available_on: NaiveDate,
    pub deposit_required: u32,
}

/// Applicant provided snapshot used to validate a submission against Fair Housing and Iowa rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSubmission {
    pub listing: VacancyListingSnapshot,
    pub household: HouseholdComposition,
    pub screening_answers: ScreeningAnswers,
    pub income: IncomeDeclaration,
    pub rental_history: Vec<RentalReference>,
    pub credit_score: Option<u16>,
    pub criminal_history: Vec<CriminalRecord>,
    pub supporting_documents: Vec<DocumentDescriptor>,
}

/// Document the household structure without capturing any protected class characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdComposition {
    pub adults: u8,
    pub children: u8,
    pub bedrooms_required: u8,
}

/// Declarative answers collected uniformly across applicants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreeningAnswers {
    pub pets: bool,
    pub service_animals: bool,
    pub smoker: bool,
    pub requested_accessibility_accommodations: Vec<String>,
    pub requested_move_in: NaiveDate,
    pub disclosed_vouchers: Vec<SubsidyProgram>,
    pub prohibited_preferences: Vec<ProhibitedScreeningPractice>,
}

/// Declared income by source to support LIHTC and subsidy documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomeDeclaration {
    pub gross_monthly_income: u32,
    pub verified_income_sources: Vec<String>,
    pub housing_voucher_amount: Option<u32>,
}

/// Historical landlord verification snapshot for verification scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RentalReference {
    pub property_name: String,
    pub paid_on_time: bool,
    pub filed_eviction: bool,
    pub tenancy_start: NaiveDate,
    pub tenancy_end: Option<NaiveDate>,
}

/// Criminal history record captured during screening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriminalRecord {
    pub classification: CriminalClassification,
    pub years_since: u8,
    pub jurisdiction: String,
    pub description: String,
}

/// Simplified criminal classifications aligned with HUD disparate impact guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CriminalClassification {
    ViolentFelony,
    NonViolentFelony,
    Misdemeanor,
}

/// Metadata for submitted proof so repositories can maintain audit trails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDescriptor {
    pub name: String,
    pub category: DocumentCategory,
    pub storage_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCategory {
    Identification,
    IncomeVerification,
    RentalReference,
    SpecialProgram,
    Misc,
}

/// Enumerates prohibited screening practices documented to align with the Fair Housing Act and the Iowa Civil Rights Act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProhibitedScreeningPractice {
    SteeringBasedOnFamilialStatus,
    SourceOfIncomeDiscrimination,
    BlanketCriminalHistoryBan,
    DisparateResponseCadence,
    ProtectedClassInquiry { field: String },
}

/// Basic subsidy descriptor so intake can track housing choice vouchers and similar programs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsidyProgram {
    pub program: String,
    pub monthly_amount: u32,
}

/// The sanitized, compliance-checked domain model after intake validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicantProfile {
    pub application_id: ApplicationId,
    pub lawful_factors: BTreeMap<LawfulFactorKind, LawfulFactorValue>,
    pub household: HouseholdComposition,
    pub listing: VacancyListingSnapshot,
    pub declared_income: IncomeDeclaration,
    pub rental_history: Vec<RentalReference>,
    pub credit_score: Option<u16>,
    pub criminal_history: Vec<CriminalRecord>,
    pub accommodations: Vec<String>,
}

/// Factors permitted in the evaluation rubric.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LawfulFactorKind {
    RentToIncome,
    CreditScore,
    RentalHistory,
    CriminalHistoryWindow,
    VoucherCoverage,
    IowaSecurityDepositCompliance,
}

/// Value representation for a lawful factor so scoring can consume structured data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LawfulFactorValue {
    Decimal(f32),
    Boolean(bool),
    Count(u32),
    Text(String),
}

/// Validation errors raised by the compliance guard.
#[derive(Debug, thiserror::Error)]
pub enum ComplianceViolation {
    #[error("submission captured prohibited screening practice: {0:?}")]
    ProhibitedPractice(ProhibitedScreeningPractice),
    #[error("security deposit exceeds Iowa two month cap (required <= {max:?}, found {found:?})")]
    IowaSecurityDepositCap { max: u32, found: u32 },
    #[error("missing verified income documentation for LIHTC/IFA requirements")]
    MissingIncomeDocumentation,
    #[error("household composition incomplete")]
    IncompleteHousehold,
}

/// Guard responsible for producing `ApplicantProfile` instances.
#[derive(Debug, Clone)]
pub struct ComplianceGuard;

impl Default for ComplianceGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceGuard {
    /// Construct a guard using policy defaults derived from the Fair Housing Act and Iowa Civil Rights Act guidance.
    pub fn new() -> Self {
        Self
    }

    /// Convert an inbound submission into a sanitized applicant profile.
    pub fn profile_from_submission(
        &self,
        submission: ApplicationSubmission,
    ) -> Result<ApplicantProfile, ComplianceViolation> {
        if let Some(prohibited) = submission
            .screening_answers
            .prohibited_preferences
            .first()
            .cloned()
        {
            return Err(ComplianceViolation::ProhibitedPractice(prohibited));
        }

        if submission.income.verified_income_sources.is_empty() {
            return Err(ComplianceViolation::MissingIncomeDocumentation);
        }

        let household = submission.household;
        if household.adults == 0 && household.children == 0 {
            return Err(ComplianceViolation::IncompleteHousehold);
        }

        let deposit_cap = submission.listing.listed_rent * 2;
        if submission.listing.deposit_required > deposit_cap {
            return Err(ComplianceViolation::IowaSecurityDepositCap {
                max: deposit_cap,
                found: submission.listing.deposit_required,
            });
        }

        let mut lawful_factors = BTreeMap::new();

        if submission.income.gross_monthly_income == 0 {
            return Err(ComplianceViolation::MissingIncomeDocumentation);
        }

        let rent_to_income =
            submission.listing.listed_rent as f32 / submission.income.gross_monthly_income as f32;
        lawful_factors.insert(
            LawfulFactorKind::RentToIncome,
            LawfulFactorValue::Decimal(rent_to_income),
        );

        if let Some(score) = submission.credit_score {
            lawful_factors.insert(
                LawfulFactorKind::CreditScore,
                LawfulFactorValue::Count(score as u32),
            );
        }

        let eviction_count = submission
            .rental_history
            .iter()
            .filter(|reference| reference.filed_eviction)
            .count() as u32;
        lawful_factors.insert(
            LawfulFactorKind::RentalHistory,
            LawfulFactorValue::Count(eviction_count),
        );

        if !submission.criminal_history.is_empty() {
            let window = submission
                .criminal_history
                .iter()
                .map(|record| record.years_since as f32)
                .fold(f32::INFINITY, f32::min);
            lawful_factors.insert(
                LawfulFactorKind::CriminalHistoryWindow,
                LawfulFactorValue::Decimal(if window.is_finite() { window } else { 0.0 }),
            );
        }

        let voucher_coverage = submission
            .income
            .housing_voucher_amount
            .map(|amount| amount as f32 / submission.listing.listed_rent as f32)
            .unwrap_or(0.0);
        lawful_factors.insert(
            LawfulFactorKind::VoucherCoverage,
            LawfulFactorValue::Decimal(voucher_coverage),
        );

        lawful_factors.insert(
            LawfulFactorKind::IowaSecurityDepositCompliance,
            LawfulFactorValue::Boolean(true),
        );

        Ok(ApplicantProfile {
            application_id: ApplicationId("pending".to_string()),
            lawful_factors,
            household,
            listing: submission.listing,
            declared_income: submission.income,
            rental_history: submission.rental_history,
            credit_score: submission.credit_score,
            criminal_history: submission.criminal_history,
            accommodations: submission
                .screening_answers
                .requested_accessibility_accommodations,
        })
    }
}

/// Rubric configuration describing the lawful scoring weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub minimum_rent_to_income_ratio: f32,
    pub minimum_credit_score: Option<u16>,
    pub max_evictions: u8,
    pub violent_felony_lookback_years: u8,
    pub non_violent_lookback_years: u8,
    pub misdemeanor_lookback_years: u8,
    pub deposit_cap_multiplier: f32,
}

/// Stateless evaluator that applies the rubric configuration to a profile.
pub struct EvaluationEngine {
    config: EvaluationConfig,
}

impl EvaluationEngine {
    pub fn new(config: EvaluationConfig) -> Self {
        Self { config }
    }

    pub fn score(&self, profile: &ApplicantProfile) -> EvaluationOutcome {
        let mut components = Vec::new();
        let mut total_score: i16 = 0;

        let rent_to_income = profile
            .lawful_factors
            .get(&LawfulFactorKind::RentToIncome)
            .and_then(|value| match value {
                LawfulFactorValue::Decimal(ratio) => Some(*ratio),
                _ => None,
            })
            .unwrap_or_else(|| {
                profile.listing.listed_rent as f32
                    / profile.declared_income.gross_monthly_income as f32
            });

        if rent_to_income <= self.config.minimum_rent_to_income_ratio {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::RentToIncome,
                score: 30,
                notes: format!(
                    "rent-to-income ratio {:.2} within policy threshold {:.2}",
                    rent_to_income, self.config.minimum_rent_to_income_ratio
                ),
            });
            total_score += 30;
        } else {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::RentToIncome,
                score: -40,
                notes: format!(
                    "ratio {:.2} exceeds required {:.2}",
                    rent_to_income, self.config.minimum_rent_to_income_ratio
                ),
            });
            total_score -= 40;
        }

        let credit_score = profile.credit_score;
        if let Some(min_credit) = self.config.minimum_credit_score {
            match credit_score {
                Some(score) if score >= min_credit => {
                    components.push(ScoreComponent {
                        factor: LawfulFactorKind::CreditScore,
                        score: 20,
                        notes: format!("credit score {score} meets minimum {min_credit}"),
                    });
                    total_score += 20;
                }
                Some(score) => {
                    components.push(ScoreComponent {
                        factor: LawfulFactorKind::CreditScore,
                        score: -25,
                        notes: format!("credit score {score} below minimum {min_credit}"),
                    });
                    total_score -= 25;
                }
                None => {
                    components.push(ScoreComponent {
                        factor: LawfulFactorKind::CreditScore,
                        score: -10,
                        notes: "missing credit history".to_string(),
                    });
                    total_score -= 10;
                }
            }
        }

        let eviction_count = profile
            .lawful_factors
            .get(&LawfulFactorKind::RentalHistory)
            .and_then(|value| match value {
                LawfulFactorValue::Count(count) => Some(*count as u8),
                _ => None,
            })
            .unwrap_or_else(|| {
                profile
                    .rental_history
                    .iter()
                    .filter(|reference| reference.filed_eviction)
                    .count() as u8
            });
        if eviction_count == 0 {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::RentalHistory,
                score: 10,
                notes: "no prior evictions".to_string(),
            });
            total_score += 10;
        } else if eviction_count <= self.config.max_evictions {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::RentalHistory,
                score: -10,
                notes: format!("{eviction_count} eviction(s) within policy"),
            });
            total_score -= 10;
        } else {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::RentalHistory,
                score: -25,
                notes: format!("{eviction_count} eviction(s) exceeds allowance"),
            });
            total_score -= 25;
        }

        if let Some(LawfulFactorValue::Decimal(coverage)) = profile
            .lawful_factors
            .get(&LawfulFactorKind::VoucherCoverage)
        {
            if *coverage > 0.0 {
                components.push(ScoreComponent {
                    factor: LawfulFactorKind::VoucherCoverage,
                    score: 5,
                    notes: format!("voucher covers {:.0}% of rent", coverage * 100.0),
                });
                total_score += 5;
            }
        }

        if let Some(LawfulFactorValue::Boolean(true)) = profile
            .lawful_factors
            .get(&LawfulFactorKind::IowaSecurityDepositCompliance)
        {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::IowaSecurityDepositCompliance,
                score: 5,
                notes: "security deposit within Iowa cap".to_string(),
            });
            total_score += 5;
        }

        let recent_violent = profile.criminal_history.iter().find(|record| {
            record.classification == CriminalClassification::ViolentFelony
                && record.years_since <= self.config.violent_felony_lookback_years
        });
        if let Some(record) = recent_violent {
            return EvaluationOutcome {
                application_id: profile.application_id.clone(),
                decision: ApplicationDecision::ManualReview {
                    reasons: vec![format!(
                        "Recent violent felony within {} years: {}",
                        self.config.violent_felony_lookback_years, record.description
                    )],
                },
                total_score,
                components,
            };
        }

        if rent_to_income > self.config.minimum_rent_to_income_ratio {
            return EvaluationOutcome {
                application_id: profile.application_id.clone(),
                decision: ApplicationDecision::Denied(DenialReason::InsufficientIncome {
                    required_ratio: self.config.minimum_rent_to_income_ratio,
                    actual_ratio: rent_to_income,
                }),
                total_score,
                components,
            };
        }

        if let Some(min_credit) = self.config.minimum_credit_score {
            if credit_score.map(|score| score < min_credit).unwrap_or(true) {
                return EvaluationOutcome {
                    application_id: profile.application_id.clone(),
                    decision: ApplicationDecision::Denied(DenialReason::AdverseCreditHistory),
                    total_score,
                    components,
                };
            }
        }

        EvaluationOutcome {
            application_id: profile.application_id.clone(),
            decision: ApplicationDecision::Approved,
            total_score,
            components,
        }
    }
}

/// Discrete contribution to an evaluation, allowing transparent audits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub factor: LawfulFactorKind,
    pub score: i16,
    pub notes: String,
}

/// Evaluation output describing the composite score and decision trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationOutcome {
    pub application_id: ApplicationId,
    pub decision: ApplicationDecision,
    pub total_score: i16,
    pub components: Vec<ScoreComponent>,
}

/// Adjudication outcome for a screened application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApplicationDecision {
    Approved,
    ConditionalApproval { required_actions: Vec<String> },
    Denied(DenialReason),
    ManualReview { reasons: Vec<String> },
}

/// Enumerates lawful denial reasons to support adverse action notices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DenialReason {
    InsufficientIncome {
        required_ratio: f32,
        actual_ratio: f32,
    },
    AdverseCreditHistory,
    ExcessiveEvictions(u8),
    CriminalDisqualifier {
        classification: CriminalClassification,
        years_since: u8,
    },
    IncompleteDocumentation,
}

/// High level status tracked throughout the vacancy application workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VacancyApplicationStatus {
    Submitted,
    UnderReview,
    Approved,
    Denied,
    Waitlisted,
}

/// Repository record containing the profile, evaluation, and status metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRecord {
    pub profile: ApplicantProfile,
    pub status: VacancyApplicationStatus,
    pub evaluation: Option<EvaluationOutcome>,
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

/// Service composing the compliance guard, repository, and evaluation rubric.
pub struct VacancyApplicationService<R, A> {
    guard: ComplianceGuard,
    repository: Arc<R>,
    alerts: Arc<A>,
    config: EvaluationConfig,
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
    pub fn new(
        guard: ComplianceGuard,
        repository: Arc<R>,
        alerts: Arc<A>,
        config: EvaluationConfig,
    ) -> Self {
        Self {
            guard,
            repository,
            alerts,
            config,
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

        let engine = EvaluationEngine::new(self.config.clone());
        let outcome = engine.score(&record.profile);

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

fn status_to_str(status: VacancyApplicationStatus) -> &'static str {
    match status {
        VacancyApplicationStatus::Submitted => "submitted",
        VacancyApplicationStatus::UnderReview => "under_review",
        VacancyApplicationStatus::Approved => "approved",
        VacancyApplicationStatus::Denied => "denied",
        VacancyApplicationStatus::Waitlisted => "waitlisted",
    }
}

fn decision_rationale(record: &ApplicationRecord) -> String {
    match &record.evaluation {
        Some(outcome) => match &outcome.decision {
            ApplicationDecision::Approved => "application approved".to_string(),
            ApplicationDecision::ConditionalApproval { required_actions } => {
                if required_actions.is_empty() {
                    "conditional approval".to_string()
                } else {
                    format!("conditional approval: {}", required_actions.join(", "))
                }
            }
            ApplicationDecision::Denied(reason) => match reason {
                DenialReason::InsufficientIncome {
                    required_ratio,
                    actual_ratio,
                } => format!(
                    "denied for insufficient income (required {:.2}, actual {:.2})",
                    required_ratio, actual_ratio
                ),
                DenialReason::AdverseCreditHistory => {
                    "denied for adverse credit history".to_string()
                }
                DenialReason::ExcessiveEvictions(count) => {
                    format!("denied for {count} eviction(s)")
                }
                DenialReason::CriminalDisqualifier {
                    classification,
                    years_since,
                } => format!("denied for {:?} {} years ago", classification, years_since),
                DenialReason::IncompleteDocumentation => {
                    "denied for incomplete documentation".to_string()
                }
            },
            ApplicationDecision::ManualReview { reasons } => {
                if reasons.is_empty() {
                    "requires manual review".to_string()
                } else {
                    format!("manual review required: {}", reasons.join("; "))
                }
            }
        },
        None => "pending evaluation".to_string(),
    }
}

async fn submit_handler<R, A>(
    State(service): State<Arc<VacancyApplicationService<R, A>>>,
    axum::Json(submission): axum::Json<ApplicationSubmission>,
) -> Response
where
    R: ApplicationRepository + 'static,
    A: AlertPublisher + 'static,
{
    match service.submit(submission) {
        Ok(record) => {
            let payload = json!({
                "application_id": record.profile.application_id.0.clone(),
                "status": status_to_str(record.status),
            });
            (StatusCode::ACCEPTED, axum::Json(payload)).into_response()
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

async fn status_handler<R, A>(
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
            let payload = json!({
                "application_id": record.profile.application_id.0.clone(),
                "status": status_to_str(record.status),
                "decision_rationale": decision_rationale(&record),
            });
            (StatusCode::OK, axum::Json(payload)).into_response()
        }
        Err(ApplicationServiceError::Repository(RepositoryError::NotFound)) => {
            let payload = json!({
                "application_id": id.0,
                "status": "pending",
                "decision_rationale": "pending evaluation",
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn listing() -> VacancyListingSnapshot {
        VacancyListingSnapshot {
            unit_id: "A-201".to_string(),
            property_code: "APOLLO".to_string(),
            listed_rent: 1180,
            available_on: NaiveDate::from_ymd_opt(2025, 10, 1).expect("valid date"),
            deposit_required: 2100,
        }
    }

    fn submission() -> ApplicationSubmission {
        ApplicationSubmission {
            listing: listing(),
            household: HouseholdComposition {
                adults: 1,
                children: 1,
                bedrooms_required: 2,
            },
            screening_answers: ScreeningAnswers {
                pets: true,
                service_animals: false,
                smoker: false,
                requested_accessibility_accommodations: vec!["Lowered countertop".to_string()],
                requested_move_in: NaiveDate::from_ymd_opt(2025, 10, 5).expect("valid"),
                disclosed_vouchers: vec![SubsidyProgram {
                    program: "HCV".to_string(),
                    monthly_amount: 450,
                }],
                prohibited_preferences: Vec::new(),
            },
            income: IncomeDeclaration {
                gross_monthly_income: 4300,
                verified_income_sources: vec!["Employer".to_string()],
                housing_voucher_amount: Some(450),
            },
            rental_history: vec![RentalReference {
                property_name: "Riverfront Lofts".to_string(),
                paid_on_time: true,
                filed_eviction: false,
                tenancy_start: NaiveDate::from_ymd_opt(2023, 9, 1).expect("valid"),
                tenancy_end: Some(NaiveDate::from_ymd_opt(2025, 8, 31).expect("valid")),
            }],
            credit_score: Some(712),
            criminal_history: vec![CriminalRecord {
                classification: CriminalClassification::Misdemeanor,
                years_since: 6,
                jurisdiction: "Polk County".to_string(),
                description: "Expired registration".to_string(),
            }],
            supporting_documents: vec![DocumentDescriptor {
                name: "Primary ID".to_string(),
                category: DocumentCategory::Identification,
                storage_key: "s3://tenant-ai/docs/app-123/id.pdf".to_string(),
            }],
        }
    }

    #[test]
    fn compliance_guard_requires_verified_income_sources() {
        let guard = ComplianceGuard::new();
        let mut submission = submission();
        submission.income.verified_income_sources.clear();

        match guard.profile_from_submission(submission) {
            Err(ComplianceViolation::MissingIncomeDocumentation) => {}
            other => panic!("expected missing income documentation, got {other:?}"),
        }
    }

    #[test]
    fn compliance_guard_rejects_zero_household_and_zero_income() {
        let guard = ComplianceGuard::new();
        let mut submission = submission();
        submission.household = HouseholdComposition {
            adults: 0,
            children: 0,
            bedrooms_required: 1,
        };
        submission.income.gross_monthly_income = 0;

        match guard.profile_from_submission(submission) {
            Err(ComplianceViolation::IncompleteHousehold) => {}
            other => panic!("expected incomplete household violation, got {other:?}"),
        }
    }

    #[test]
    fn compliance_guard_flags_zero_income_even_with_verified_sources() {
        let guard = ComplianceGuard::new();
        let mut submission = submission();
        submission.income.gross_monthly_income = 0;

        match guard.profile_from_submission(submission) {
            Err(ComplianceViolation::MissingIncomeDocumentation) => {}
            other => panic!("expected missing income documentation, got {other:?}"),
        }
    }

    #[test]
    fn evaluation_engine_denies_for_low_credit_history() {
        let engine = EvaluationEngine::new(EvaluationConfig {
            minimum_rent_to_income_ratio: 0.3,
            minimum_credit_score: Some(650),
            max_evictions: 1,
            violent_felony_lookback_years: 7,
            non_violent_lookback_years: 5,
            misdemeanor_lookback_years: 3,
            deposit_cap_multiplier: 2.0,
        });
        let profile = guard_profile("credit-low", 0.29, Some(610));

        let outcome = engine.score(&profile);

        match outcome.decision {
            ApplicationDecision::Denied(DenialReason::AdverseCreditHistory) => {}
            other => panic!("expected adverse credit denial, got {other:?}"),
        }
        assert!(outcome
            .components
            .iter()
            .any(|component| component.factor == LawfulFactorKind::CreditScore));
    }

    #[test]
    fn evaluation_engine_handles_missing_credit_history() {
        let engine = EvaluationEngine::new(EvaluationConfig {
            minimum_rent_to_income_ratio: 0.35,
            minimum_credit_score: Some(600),
            max_evictions: 1,
            violent_felony_lookback_years: 7,
            non_violent_lookback_years: 5,
            misdemeanor_lookback_years: 3,
            deposit_cap_multiplier: 2.0,
        });
        let profile = guard_profile("credit-missing", 0.3, None);

        let outcome = engine.score(&profile);

        match outcome.decision {
            ApplicationDecision::Denied(DenialReason::AdverseCreditHistory) => {}
            other => panic!("expected adverse credit denial, got {other:?}"),
        }
        assert!(outcome
            .components
            .iter()
            .any(|component| component.factor == LawfulFactorKind::CreditScore && component.score < 0));
    }

    #[test]
    fn service_submit_propagates_compliance_errors() {
        let guard = ComplianceGuard::new();
        let repository = Arc::new(MemoryRepository::default());
        let alerts = Arc::new(MemoryAlerts::default());
        let service = VacancyApplicationService::new(
            guard,
            repository,
            alerts,
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        );

        let mut submission = submission();
        submission.income.verified_income_sources.clear();

        match service.submit(submission) {
            Err(ApplicationServiceError::Compliance(
                ComplianceViolation::MissingIncomeDocumentation,
            )) => {}
            other => panic!("expected compliance violation, got {other:?}"),
        }
    }

    #[test]
    fn service_evaluate_sets_under_review_on_manual_review_outcomes() {
        let guard = ComplianceGuard::new();
        let repository = Arc::new(MemoryRepository::default());
        let alerts = Arc::new(MemoryAlerts::default());
        let config = EvaluationConfig {
            minimum_rent_to_income_ratio: 0.3,
            minimum_credit_score: Some(600),
            max_evictions: 1,
            violent_felony_lookback_years: 7,
            non_violent_lookback_years: 5,
            misdemeanor_lookback_years: 3,
            deposit_cap_multiplier: 2.0,
        };
        let service = VacancyApplicationService::new(
            guard,
            repository.clone(),
            alerts.clone(),
            config,
        );

        let mut submission = submission();
        submission.criminal_history.push(CriminalRecord {
            classification: CriminalClassification::ViolentFelony,
            years_since: 2,
            jurisdiction: "Polk County".to_string(),
            description: "Assault".to_string(),
        });

        let record = service
            .submit(submission)
            .expect("can submit manual review candidate");
        let outcome = service
            .evaluate(&record.profile.application_id)
            .expect("manual review outcome");

        assert!(matches!(outcome.decision, ApplicationDecision::ManualReview { .. }));
        let stored = repository
            .fetch(&record.profile.application_id)
            .expect("fetch succeeds")
            .expect("record present");
        assert_eq!(stored.status, VacancyApplicationStatus::UnderReview);
        assert!(alerts.events().is_empty(), "manual review should not emit alerts");
    }

    #[test]
    fn service_get_propagates_not_found() {
        let guard = ComplianceGuard::new();
        let repository = Arc::new(MemoryRepository::default());
        let alerts = Arc::new(MemoryAlerts::default());
        let service = VacancyApplicationService::new(
            guard,
            repository,
            alerts,
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        );

        match service.get(&ApplicationId("missing".to_string())) {
            Err(ApplicationServiceError::Repository(RepositoryError::NotFound)) => {}
            other => panic!("expected not found error, got {other:?}"),
        }
    }

    #[test]
    fn status_to_str_matches_each_variant() {
        assert_eq!(status_to_str(VacancyApplicationStatus::Submitted), "submitted");
        assert_eq!(
            status_to_str(VacancyApplicationStatus::UnderReview),
            "under_review"
        );
        assert_eq!(status_to_str(VacancyApplicationStatus::Approved), "approved");
        assert_eq!(status_to_str(VacancyApplicationStatus::Denied), "denied");
        assert_eq!(status_to_str(VacancyApplicationStatus::Waitlisted), "waitlisted");
    }

    #[test]
    fn decision_rationale_formats_outcomes() {
        let id = ApplicationId("app-123".to_string());
        let profile = guard_profile("rationale", 0.25, Some(700));

        let approved = ApplicationRecord {
            profile: profile.clone(),
            status: VacancyApplicationStatus::Approved,
            evaluation: Some(EvaluationOutcome {
                application_id: id.clone(),
                decision: ApplicationDecision::Approved,
                total_score: 42,
                components: Vec::new(),
            }),
        };
        assert!(decision_rationale(&approved).contains("approved"));

        let conditional = ApplicationRecord {
            profile: profile.clone(),
            status: VacancyApplicationStatus::UnderReview,
            evaluation: Some(EvaluationOutcome {
                application_id: id.clone(),
                decision: ApplicationDecision::ConditionalApproval {
                    required_actions: vec!["provide insurance".to_string()],
                },
                total_score: 10,
                components: Vec::new(),
            }),
        };
        assert!(decision_rationale(&conditional).contains("conditional"));

        let denied = ApplicationRecord {
            profile: profile.clone(),
            status: VacancyApplicationStatus::Denied,
            evaluation: Some(EvaluationOutcome {
                application_id: id.clone(),
                decision: ApplicationDecision::Denied(DenialReason::InsufficientIncome {
                    required_ratio: 0.3,
                    actual_ratio: 0.45,
                }),
                total_score: -10,
                components: Vec::new(),
            }),
        };
        assert!(decision_rationale(&denied).contains("insufficient income"));

        let manual = ApplicationRecord {
            profile: profile.clone(),
            status: VacancyApplicationStatus::UnderReview,
            evaluation: Some(EvaluationOutcome {
                application_id: id.clone(),
                decision: ApplicationDecision::ManualReview {
                    reasons: vec!["income discrepancy".to_string()],
                },
                total_score: 0,
                components: Vec::new(),
            }),
        };
        assert!(decision_rationale(&manual).contains("manual review"));

        let pending = ApplicationRecord {
            profile,
            status: VacancyApplicationStatus::Submitted,
            evaluation: None,
        };
        assert_eq!(decision_rationale(&pending), "pending evaluation");
    }

    #[tokio::test]
    async fn submit_handler_returns_conflict_on_duplicate() {
        let service = Arc::new(VacancyApplicationService::new(
            ComplianceGuard::new(),
            Arc::new(ConflictRepository),
            Arc::new(MemoryAlerts::default()),
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        ));

        let response = submit_handler(State(service), axum::Json(submission())).await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn submit_handler_returns_unprocessable_for_compliance_error() {
        let service = Arc::new(VacancyApplicationService::new(
            ComplianceGuard::new(),
            Arc::new(MemoryRepository::default()),
            Arc::new(MemoryAlerts::default()),
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        ));

        let mut bad_submission = submission();
        bad_submission.income.verified_income_sources.clear();

        let response = submit_handler(State(service), axum::Json(bad_submission)).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn submit_handler_returns_internal_error_on_repository_failure() {
        let service = Arc::new(VacancyApplicationService::new(
            ComplianceGuard::new(),
            Arc::new(UnavailableRepository),
            Arc::new(MemoryAlerts::default()),
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        ));

        let response = submit_handler(State(service), axum::Json(submission())).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn status_handler_returns_internal_error_on_repository_failure() {
        let service = Arc::new(VacancyApplicationService::new(
            ComplianceGuard::new(),
            Arc::new(UnavailableRepository),
            Arc::new(MemoryAlerts::default()),
            EvaluationConfig {
                minimum_rent_to_income_ratio: 0.3,
                minimum_credit_score: Some(600),
                max_evictions: 1,
                violent_felony_lookback_years: 7,
                non_violent_lookback_years: 5,
                misdemeanor_lookback_years: 3,
                deposit_cap_multiplier: 2.0,
            },
        ));

        let response = status_handler(State(service), Path("app-unknown".to_string())).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn status_handler_formats_success_payload() {
        let repository = Arc::new(MemoryRepository::default());
        let alerts = Arc::new(MemoryAlerts::default());
        let guard = ComplianceGuard::new();
        let config = EvaluationConfig {
            minimum_rent_to_income_ratio: 0.3,
            minimum_credit_score: Some(600),
            max_evictions: 1,
            violent_felony_lookback_years: 7,
            non_violent_lookback_years: 5,
            misdemeanor_lookback_years: 3,
            deposit_cap_multiplier: 2.0,
        };
        let service = Arc::new(VacancyApplicationService::new(
            guard,
            repository.clone(),
            alerts,
            config,
        ));

        let record = service.submit(submission()).expect("submission succeeds");

        let response = status_handler(
            State(service),
            Path(record.profile.application_id.0.clone()),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload.get("application_id").and_then(Value::as_str),
            Some(record.profile.application_id.0.as_str())
        );
        assert!(payload
            .get("decision_rationale")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("pending"));
    }

    #[derive(Default, Clone)]
    struct MemoryRepository {
        records: Arc<Mutex<HashMap<ApplicationId, ApplicationRecord>>>,
    }

    impl ApplicationRepository for MemoryRepository {
        fn insert(&self, record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError> {
            let mut guard = self.records.lock().expect("lock");
            if guard.contains_key(&record.profile.application_id) {
                return Err(RepositoryError::Conflict);
            }
            guard.insert(record.profile.application_id.clone(), record.clone());
            Ok(record)
        }

        fn update(&self, record: ApplicationRecord) -> Result<(), RepositoryError> {
            let mut guard = self.records.lock().expect("lock");
            guard.insert(record.profile.application_id.clone(), record);
            Ok(())
        }

        fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
            let guard = self.records.lock().expect("lock");
            Ok(guard.get(id).cloned())
        }

        fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
            Ok(Vec::new())
        }
    }

    #[derive(Default, Clone)]
    struct MemoryAlerts {
        events: Arc<Mutex<Vec<AppFolioAlert>>>,
    }

    impl MemoryAlerts {
        fn events(&self) -> Vec<AppFolioAlert> {
            self.events.lock().expect("lock").clone()
        }
    }

    impl AlertPublisher for MemoryAlerts {
        fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError> {
            self.events.lock().expect("lock").push(alert);
            Ok(())
        }
    }

    struct ConflictRepository;

    impl ApplicationRepository for ConflictRepository {
        fn insert(&self, _record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError> {
            Err(RepositoryError::Conflict)
        }

        fn update(&self, _record: ApplicationRecord) -> Result<(), RepositoryError> {
            Err(RepositoryError::Unavailable("read only".to_string()))
        }

        fn fetch(&self, _id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
            Ok(None)
        }

        fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
            Ok(Vec::new())
        }
    }

    struct UnavailableRepository;

    impl ApplicationRepository for UnavailableRepository {
        fn insert(&self, _record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError> {
            Err(RepositoryError::Unavailable("database offline".to_string()))
        }

        fn update(&self, _record: ApplicationRecord) -> Result<(), RepositoryError> {
            Err(RepositoryError::Unavailable("database offline".to_string()))
        }

        fn fetch(&self, _id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
            Err(RepositoryError::Unavailable("database offline".to_string()))
        }

        fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
            Err(RepositoryError::Unavailable("database offline".to_string()))
        }
    }

    fn guard_profile(
        suffix: &str,
        ratio: f32,
        credit_score: Option<u16>,
    ) -> ApplicantProfile {
        let mut lawful_factors = BTreeMap::new();
        lawful_factors.insert(
            LawfulFactorKind::RentToIncome,
            LawfulFactorValue::Decimal(ratio),
        );
        lawful_factors.insert(
            LawfulFactorKind::VoucherCoverage,
            LawfulFactorValue::Decimal(0.3),
        );
        lawful_factors.insert(
            LawfulFactorKind::IowaSecurityDepositCompliance,
            LawfulFactorValue::Boolean(true),
        );
        lawful_factors.insert(
            LawfulFactorKind::RentalHistory,
            LawfulFactorValue::Count(0),
        );
        if let Some(score) = credit_score {
            lawful_factors.insert(
                LawfulFactorKind::CreditScore,
                LawfulFactorValue::Count(score as u32),
            );
        }

        ApplicantProfile {
            application_id: ApplicationId(format!("app-{suffix}")),
            lawful_factors,
            household: HouseholdComposition {
                adults: 1,
                children: 0,
                bedrooms_required: 1,
            },
            listing: listing(),
            declared_income: IncomeDeclaration {
                gross_monthly_income: 4300,
                verified_income_sources: vec!["Employer".to_string()],
                housing_voucher_amount: Some(300),
            },
            rental_history: vec![RentalReference {
                property_name: "Riverfront Lofts".to_string(),
                paid_on_time: true,
                filed_eviction: false,
                tenancy_start: NaiveDate::from_ymd_opt(2023, 1, 1).expect("valid"),
                tenancy_end: None,
            }],
            credit_score,
            criminal_history: Vec::new(),
            accommodations: vec!["First floor".to_string()],
        }
    }
}
