//! Integration specifications for the vacancy application intake and evaluation workflow.
//!
//! These tests are intentionally written ahead of the implementation so that they can guide
//! the forthcoming TDD cycles. Each scenario captures compliance, scoring, service orchestration,
//! and HTTP contract expectations derived from the Fair Housing Act and Iowa-specific guidance.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::NaiveDate;
use serde_json::json;
use tower::ServiceExt;

use tenant_ai::workflows::vacancy::applications::{
    application_router, AppFolioAlert, ApplicantProfile, ApplicationDecision, ApplicationId,
    ApplicationRecord, ApplicationRepository, ApplicationSubmission, ComplianceGuard,
    ComplianceViolation, CriminalClassification, CriminalRecord, DenialReason, DocumentCategory,
    DocumentDescriptor, EvaluationConfig, EvaluationEngine, HouseholdComposition,
    IncomeDeclaration, LawfulFactorKind, LawfulFactorValue, ProhibitedScreeningPractice,
    RentalReference, RepositoryError, ScreeningAnswers, SubsidyProgram, VacancyApplicationService,
    VacancyApplicationStatus, VacancyListingSnapshot,
};

// -- Helpers ----------------------------------------------------------------

fn listing() -> VacancyListingSnapshot {
    VacancyListingSnapshot {
        unit_id: "A-201".to_string(),
        property_code: "APOLLO".to_string(),
        listed_rent: 1180,
        available_on: NaiveDate::from_ymd_opt(2025, 10, 1).expect("valid date"),
        deposit_required: 2100,
    }
}

fn rental_history() -> Vec<RentalReference> {
    vec![RentalReference {
        property_name: "Riverfront Lofts".to_string(),
        paid_on_time: true,
        filed_eviction: false,
        tenancy_start: NaiveDate::from_ymd_opt(2023, 9, 1).expect("valid"),
        tenancy_end: Some(NaiveDate::from_ymd_opt(2025, 8, 31).expect("valid")),
    }]
}

fn documents() -> Vec<DocumentDescriptor> {
    vec![
        DocumentDescriptor {
            name: "Primary ID".to_string(),
            category: DocumentCategory::Identification,
            storage_key: "s3://tenant-ai/docs/app-123/id.pdf".to_string(),
        },
        DocumentDescriptor {
            name: "VOE".to_string(),
            category: DocumentCategory::IncomeVerification,
            storage_key: "s3://tenant-ai/docs/app-123/voe.pdf".to_string(),
        },
    ]
}

fn screening_answers() -> ScreeningAnswers {
    ScreeningAnswers {
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
    }
}

fn income() -> IncomeDeclaration {
    IncomeDeclaration {
        gross_monthly_income: 4300,
        verified_income_sources: vec!["Employer VOE".to_string(), "SSI".to_string()],
        housing_voucher_amount: Some(450),
    }
}

fn criminal_history() -> Vec<CriminalRecord> {
    vec![CriminalRecord {
        classification: CriminalClassification::Misdemeanor,
        years_since: 6,
        jurisdiction: "Polk County".to_string(),
        description: "Expired vehicle registration".to_string(),
    }]
}

fn submission() -> ApplicationSubmission {
    ApplicationSubmission {
        listing: listing(),
        household: HouseholdComposition {
            adults: 1,
            children: 1,
            bedrooms_required: 2,
        },
        screening_answers: screening_answers(),
        income: income(),
        rental_history: rental_history(),
        credit_score: Some(712),
        criminal_history: criminal_history(),
        supporting_documents: documents(),
    }
}

fn evaluation_config() -> EvaluationConfig {
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

fn profile_for_scoring(id_suffix: &str) -> ApplicantProfile {
    let mut lawful_factors = BTreeMap::new();
    lawful_factors.insert(
        LawfulFactorKind::RentToIncome,
        LawfulFactorValue::Decimal(0.27),
    );
    lawful_factors.insert(LawfulFactorKind::CreditScore, LawfulFactorValue::Count(712));
    lawful_factors.insert(LawfulFactorKind::RentalHistory, LawfulFactorValue::Count(0));
    lawful_factors.insert(
        LawfulFactorKind::CriminalHistoryWindow,
        LawfulFactorValue::Decimal(6.0),
    );
    lawful_factors.insert(
        LawfulFactorKind::VoucherCoverage,
        LawfulFactorValue::Decimal(0.38),
    );
    lawful_factors.insert(
        LawfulFactorKind::IowaSecurityDepositCompliance,
        LawfulFactorValue::Boolean(true),
    );

    ApplicantProfile {
        application_id: ApplicationId(format!("app-{id_suffix}")),
        lawful_factors,
        household: HouseholdComposition {
            adults: 1,
            children: 1,
            bedrooms_required: 2,
        },
        listing: listing(),
        declared_income: income(),
        rental_history: rental_history(),
        credit_score: Some(712),
        criminal_history: criminal_history(),
        accommodations: vec!["Lowered countertop".to_string()],
    }
}

// -- Compliance guard specifications ----------------------------------------

/// FHA (42 U.S.C. § 3604) and Iowa Civil Rights Act prohibit inquiries into disability, familial status, etc.
#[test]
fn compliance_guard_rejects_protected_class_inquiries() {
    let guard = ComplianceGuard::new();
    let mut submission = submission();
    submission.screening_answers.prohibited_preferences.push(
        ProhibitedScreeningPractice::ProtectedClassInquiry {
            field: "disability".to_string(),
        },
    );

    let outcome = guard.profile_from_submission(submission);

    match outcome {
        Err(ComplianceViolation::ProhibitedPractice(
            ProhibitedScreeningPractice::ProtectedClassInquiry { field },
        )) => {
            assert_eq!(field, "disability");
        }
        other => panic!("expected protected class violation, got {other:?}"),
    }
}

/// Iowa Code § 562A.12 caps security deposits at two months' rent; the guard should surface violations.
#[test]
fn compliance_guard_enforces_iowa_security_deposit_cap() {
    let guard = ComplianceGuard::new();
    let mut submission = submission();
    submission.listing.deposit_required = submission.listing.listed_rent * 3;
    let listing = submission.listing.clone();

    let outcome = guard.profile_from_submission(submission);

    match outcome {
        Err(ComplianceViolation::IowaSecurityDepositCap { max, found }) => {
            assert_eq!(max, listing.listed_rent * 2);
            assert_eq!(found, listing.listed_rent * 3);
        }
        other => panic!("expected security deposit violation, got {other:?}"),
    }
}

/// Successful intake should yield a sanitized, lawful-factor aware profile ready for scoring.
#[test]
fn compliance_guard_produces_lawful_factor_profile() {
    let guard = ComplianceGuard::new();
    let submission = submission();

    let profile = guard
        .profile_from_submission(submission)
        .expect("expected compliant submission");

    assert_eq!(profile.household.adults, 1);
    assert!(profile
        .lawful_factors
        .contains_key(&LawfulFactorKind::RentToIncome));
    assert!(profile
        .lawful_factors
        .contains_key(&LawfulFactorKind::VoucherCoverage));
    assert!(
        !profile
            .lawful_factors
            .contains_key(&LawfulFactorKind::CreditScore)
            || profile.credit_score.is_some()
    );
}

// -- Evaluation rubric specifications ---------------------------------------

/// Weighted scoring must reward low rent-to-income ratios, solid credit, and clean rental history.
#[test]
fn evaluation_engine_awards_points_for_lawful_strengths() {
    let engine = EvaluationEngine::new(evaluation_config());
    let profile = profile_for_scoring("strong");

    let outcome = engine.score(&profile);

    assert_eq!(outcome.application_id, profile.application_id);
    assert!(matches!(outcome.decision, ApplicationDecision::Approved));
    assert!(outcome.components.iter().any(|component| component.factor
        == LawfulFactorKind::RentToIncome
        && component.score > 0));
    assert!(outcome.total_score > 0);
}

/// Income ratios above the policy threshold must yield lawful denial reasons suitable for adverse action notices.
#[test]
fn evaluation_engine_denies_when_rent_to_income_is_too_high() {
    let engine = EvaluationEngine::new(evaluation_config());
    let mut profile = profile_for_scoring("ratio-high");
    profile.lawful_factors.insert(
        LawfulFactorKind::RentToIncome,
        LawfulFactorValue::Decimal(0.45),
    );

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::Denied(DenialReason::InsufficientIncome {
            required_ratio,
            actual_ratio,
        }) => {
            assert_eq!(
                required_ratio,
                evaluation_config().minimum_rent_to_income_ratio
            );
            assert_eq!(actual_ratio, 0.45);
        }
        other => panic!("expected insufficient income denial, got {other:?}"),
    }
}

/// Violent felonies inside the lookback window should not auto-approve and need manual review/denial.
#[test]
fn evaluation_engine_routes_recent_violent_felonies_to_manual_review() {
    let engine = EvaluationEngine::new(evaluation_config());
    let mut profile = profile_for_scoring("felony");
    profile.criminal_history.push(CriminalRecord {
        classification: CriminalClassification::ViolentFelony,
        years_since: 3,
        jurisdiction: "Iowa District Court".to_string(),
        description: "Assault causing serious injury".to_string(),
    });

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::ManualReview { reasons } => {
            assert!(reasons
                .iter()
                .any(|reason| reason.to_lowercase().contains("violent felony")));
        }
        other => panic!("expected manual review, got {other:?}"),
    }
}

// -- Service module specifications ------------------------------------------

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
        if guard.contains_key(&record.profile.application_id) {
            guard.insert(record.profile.application_id.clone(), record);
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("lock");
        Ok(guard.get(id).cloned())
    }

    fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("lock");
        Ok(guard
            .values()
            .filter(|record| record.status == VacancyApplicationStatus::UnderReview)
            .cloned()
            .collect())
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

impl tenant_ai::workflows::vacancy::applications::AlertPublisher for MemoryAlerts {
    fn publish(
        &self,
        alert: AppFolioAlert,
    ) -> Result<(), tenant_ai::workflows::vacancy::applications::AlertError> {
        self.events.lock().expect("lock").push(alert);
        Ok(())
    }
}

/// Service should persist submissions, update statuses, and emit AppFolio alerts for approvals.
#[test]
fn service_submits_evaluates_and_emits_alerts() {
    let guard = ComplianceGuard::new();
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service = VacancyApplicationService::new(
        guard,
        repository.clone(),
        alerts.clone(),
        evaluation_config(),
    );

    let record = service
        .submit(submission())
        .expect("submit should store application");
    assert_eq!(record.status, VacancyApplicationStatus::Submitted);

    let evaluation = service
        .evaluate(&record.profile.application_id)
        .expect("evaluation should complete");
    assert!(matches!(
        evaluation.decision,
        ApplicationDecision::Approved | ApplicationDecision::ConditionalApproval { .. }
    ));

    let stored = repository
        .fetch(&record.profile.application_id)
        .expect("fetch succeeds")
        .expect("record present");
    assert!(matches!(
        stored.status,
        VacancyApplicationStatus::Approved | VacancyApplicationStatus::UnderReview
    ));

    let alerts = alerts.events();
    assert!(alerts
        .iter()
        .any(|alert| alert.template.contains("applicant_approved")));
}

/// Denied outcomes must be persisted with explicit lawful reasons for adverse action documentation.
#[test]
fn service_records_denials_with_lawful_reasons() {
    let guard = ComplianceGuard::new();
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service = VacancyApplicationService::new(
        guard,
        repository.clone(),
        alerts.clone(),
        evaluation_config(),
    );

    let mut high_rent_submission = submission();
    high_rent_submission.listing.listed_rent = 3200;
    high_rent_submission.listing.deposit_required = 6400;

    let record = service
        .submit(high_rent_submission)
        .expect("submit should store application");

    let evaluation = service
        .evaluate(&record.profile.application_id)
        .expect("evaluation should complete");

    match evaluation.decision {
        ApplicationDecision::Denied(DenialReason::InsufficientIncome { .. }) => {}
        other => panic!("expected income denial, got {other:?}"),
    }

    let stored = repository
        .fetch(&record.profile.application_id)
        .expect("fetch succeeds")
        .expect("record present");
    assert_eq!(stored.status, VacancyApplicationStatus::Denied);
}

// -- HTTP contract specifications -------------------------------------------

fn build_router() -> axum::Router {
    let guard = ComplianceGuard::new();
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service = Arc::new(VacancyApplicationService::new(
        guard,
        repository,
        alerts,
        evaluation_config(),
    ));
    application_router(service)
}

/// `POST /api/v1/vacancy/applications` should accept submissions and return 202 with tracking metadata.
#[tokio::test]
async fn post_applications_returns_tracking_id() {
    let router = build_router();
    let submission = submission();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/vacancy/applications")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&submission).expect("serialize"),
        ))
        .expect("request");

    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("router dispatch");

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert!(payload.get("application_id").is_some());
    assert_eq!(
        payload.get("status").and_then(|status| status.as_str()),
        Some("submitted")
    );
}

/// `GET /api/v1/vacancy/applications/:id` should surface the evaluation summary and lawful rationale.
#[tokio::test]
async fn get_application_returns_status_snapshot() {
    let router = build_router();
    let application_id = "app-abc123";
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/vacancy/applications/{application_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router dispatch");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload.get("application_id"), Some(&json!(application_id)));
    assert!(payload.get("decision_rationale").is_some());
}
