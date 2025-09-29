//! Integration specifications for the vacancy application intake and evaluation workflow.
//!
//! Scenarios focus on end-to-end behavior delivered through the public service facade and HTTP
//! router so we can validate compliance, evaluation, and routing without reaching into private
//! modules.

mod common {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use chrono::NaiveDate;

    use tenant_ai::workflows::vacancy::applications::domain::{
        ApplicationId, ApplicationSubmission, CriminalClassification, CriminalRecord,
        DocumentCategory, DocumentDescriptor, HouseholdComposition, IncomeDeclaration,
        RentalReference, ScreeningAnswers, SubsidyProgram, VacancyListingSnapshot,
    };
    use tenant_ai::workflows::vacancy::applications::repository::{
        AlertError, AlertPublisher, AppFolioAlert, ApplicationRepository, RepositoryError,
    };
    use tenant_ai::workflows::vacancy::applications::{
        ApplicationRecord, EvaluationConfig, VacancyApplicationService,
    };

    pub(super) fn listing() -> VacancyListingSnapshot {
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

    pub(super) fn submission() -> ApplicationSubmission {
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

    pub(super) fn evaluation_config() -> EvaluationConfig {
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

    #[derive(Default, Clone)]
    pub(super) struct MemoryRepository {
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
    pub(super) struct MemoryAlerts {
        events: Arc<Mutex<Vec<AppFolioAlert>>>,
    }

    impl MemoryAlerts {
        pub(super) fn events(&self) -> Vec<AppFolioAlert> {
            self.events.lock().expect("lock").clone()
        }
    }

    impl AlertPublisher for MemoryAlerts {
        fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError> {
            self.events.lock().expect("lock").push(alert);
            Ok(())
        }
    }

    pub(super) fn build_service() -> (
        VacancyApplicationService<MemoryRepository, MemoryAlerts>,
        Arc<MemoryRepository>,
        Arc<MemoryAlerts>,
    ) {
        let repository = Arc::new(MemoryRepository::default());
        let alerts = Arc::new(MemoryAlerts::default());
        let service =
            VacancyApplicationService::new(repository.clone(), alerts.clone(), evaluation_config());
        (service, repository, alerts)
    }

    pub(super) use MemoryAlerts as Alerts;
    pub(super) use MemoryRepository as Repository;
}

mod compliance {
    use super::common::*;
    use tenant_ai::workflows::vacancy::applications::domain::LawfulFactorKind;
    use tenant_ai::workflows::vacancy::applications::{
        ApplicationRepository, ApplicationServiceError, ProhibitedScreeningPractice,
        VacancyApplicationStatus,
    };

    #[test]
    fn prohibited_preferences_trigger_compliance_error() {
        let (service, _, _) = build_service();
        let mut bad_submission = submission();
        bad_submission
            .screening_answers
            .prohibited_preferences
            .push(ProhibitedScreeningPractice::ProtectedClassInquiry {
                field: "disability".to_string(),
            });

        match service.submit(bad_submission) {
            Err(ApplicationServiceError::Compliance(err)) => {
                assert!(err.to_string().to_lowercase().contains("protected"));
            }
            other => panic!("expected compliance violation, got {other:?}"),
        }
    }

    #[test]
    fn security_deposit_violation_flagged() {
        let (service, _, _) = build_service();
        let mut bad_submission = submission();
        bad_submission.listing.deposit_required = bad_submission.listing.listed_rent * 3;

        match service.submit(bad_submission) {
            Err(ApplicationServiceError::Compliance(err)) => {
                let message = err.to_string();
                assert!(message.contains("Iowa") || message.contains("cap"));
            }
            other => panic!("expected security deposit cap violation, got {other:?}"),
        }
    }

    #[test]
    fn submission_profiles_include_lawful_factors() {
        let (service, repository, _) = build_service();
        let record = service
            .submit(submission())
            .expect("submission should succeed");
        let stored = repository
            .fetch(&record.profile.application_id)
            .expect("repo fetch")
            .expect("record present");

        assert!(stored
            .profile
            .lawful_factors
            .contains_key(&LawfulFactorKind::RentToIncome));
        assert_eq!(stored.status, VacancyApplicationStatus::Submitted);
    }
}

mod evaluation {
    use super::common::*;
    use tenant_ai::workflows::vacancy::applications::domain::{
        CriminalClassification, CriminalRecord,
    };
    use tenant_ai::workflows::vacancy::applications::{
        ApplicationDecision, ApplicationRepository, DenialReason, VacancyApplicationStatus,
    };

    #[test]
    fn high_strength_profile_is_approved() {
        let (service, _, _) = build_service();
        let record = service.submit(submission()).expect("submission succeeds");
        let outcome = service
            .evaluate(&record.profile.application_id)
            .expect("evaluation succeeds");
        assert!(matches!(outcome.decision, ApplicationDecision::Approved));
    }

    #[test]
    fn rent_to_income_denial_is_returned() {
        let (service, repository, _) = build_service();
        let mut high_rent_submission = submission();
        high_rent_submission.listing.listed_rent = 3200;
        high_rent_submission.listing.deposit_required = 6400;

        let record = service
            .submit(high_rent_submission)
            .expect("submission stored");
        let outcome = service
            .evaluate(&record.profile.application_id)
            .expect("evaluation");

        match outcome.decision {
            ApplicationDecision::Denied(DenialReason::InsufficientIncome {
                actual_ratio, ..
            }) => {
                assert!(actual_ratio > evaluation_config().minimum_rent_to_income_ratio);
            }
            other => panic!("expected insufficient income denial, got {other:?}"),
        }

        let stored = repository
            .fetch(&record.profile.application_id)
            .expect("repo fetch")
            .expect("record present");
        assert_eq!(stored.status, VacancyApplicationStatus::Denied);
    }

    #[test]
    fn violent_felony_routes_to_manual_review() {
        let (service, _, _) = build_service();
        let mut submission = submission();
        submission.criminal_history.push(CriminalRecord {
            classification: CriminalClassification::ViolentFelony,
            years_since: 2,
            jurisdiction: "Polk County".to_string(),
            description: "Assault".to_string(),
        });

        let record = service.submit(submission).expect("submission");
        let outcome = service
            .evaluate(&record.profile.application_id)
            .expect("evaluation");

        assert!(matches!(
            outcome.decision,
            ApplicationDecision::ManualReview { .. }
        ));
    }
}

mod routing {
    use super::common::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tenant_ai::workflows::vacancy::applications::repository::ApplicationRecord;
    use tenant_ai::workflows::vacancy::applications::{
        application_router, ApplicationDecision, ApplicationRepository, EvaluationOutcome,
        VacancyApplicationService, VacancyApplicationStatus,
    };
    use tower::ServiceExt;

    fn build_router() -> axum::Router {
        let repository = Arc::new(Repository::default());
        let alerts = Arc::new(Alerts::default());
        let service = Arc::new(VacancyApplicationService::new(
            repository,
            alerts,
            evaluation_config(),
        ));
        application_router(service)
    }

    #[tokio::test]
    async fn post_applications_returns_tracking_id() {
        let router = build_router();
        let submission = submission();

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/vacancy/applications")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&submission).expect("serialize submission"),
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
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert!(payload.get("application_id").is_some());
        assert_eq!(
            payload.get("status").and_then(|status| status.as_str()),
            Some("submitted"),
        );
    }

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
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload.get("application_id"), Some(&json!(application_id)));
        assert!(payload.get("decision_rationale").is_some());
    }

    #[tokio::test]
    async fn get_application_returns_persisted_record() {
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

        let router = application_router(service.clone());
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/vacancy/applications/{}",
                        record.profile.application_id.0
                    ))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router dispatch");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload.get("application_id").and_then(Value::as_str),
            Some(record.profile.application_id.0.as_str())
        );
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some(VacancyApplicationStatus::Approved.label()),
        );
        assert_eq!(payload.get("total_score").and_then(Value::as_i64), Some(55));
        assert!(alerts.events().is_empty());
    }

    #[tokio::test]
    async fn get_application_returns_pending_view_when_missing() {
        let (service, repository, alerts) = build_service();
        let service = Arc::new(service);
        let record = service.submit(submission()).expect("submission succeeds");

        let router = application_router(service);
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/v1/vacancy/applications/{}-missing",
                        record.profile.application_id.0
                    ))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router dispatch");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(payload.get("status"), Some(&json!("submitted")));
        assert!(matches!(
            payload.get("total_score"),
            None | Some(Value::Null)
        ));
        assert!(payload
            .get("decision_rationale")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("pending"));

        assert!(repository.pending(10).unwrap().is_empty());
        assert!(alerts.events().is_empty());
    }
}
