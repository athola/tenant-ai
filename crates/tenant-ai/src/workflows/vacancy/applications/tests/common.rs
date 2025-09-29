use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use axum::response::Response;
use chrono::NaiveDate;
use serde_json::Value;

use crate::workflows::vacancy::applications::compliance::ComplianceGuard;
use crate::workflows::vacancy::applications::domain::{
    ApplicantProfile, ApplicationId, ApplicationSubmission, CriminalClassification, CriminalRecord,
    DocumentCategory, DocumentDescriptor, HouseholdComposition, IncomeDeclaration,
    LawfulFactorKind, LawfulFactorValue, ProhibitedScreeningPractice, RentalReference,
    ScreeningAnswers, SubsidyProgram, VacancyListingSnapshot,
};
use crate::workflows::vacancy::applications::evaluation::EvaluationEngine;
use crate::workflows::vacancy::applications::repository::{
    AlertError, AlertPublisher, AppFolioAlert, ApplicationRecord, ApplicationRepository,
    RepositoryError,
};
use crate::workflows::vacancy::applications::{
    application_router, EvaluationConfig, VacancyApplicationService,
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

pub(super) fn evaluation_config() -> EvaluationConfig {
    EvaluationConfig {
        minimum_rent_to_income_ratio: 0.3,
        minimum_credit_score: Some(600),
        max_evictions: 1,
        violent_felony_lookback_years: 7,
        non_violent_lookback_years: 5,
        misdemeanor_lookback_years: 3,
        deposit_cap_multiplier: 2.0,
    }
}

pub(super) fn submission() -> ApplicationSubmission {
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

pub(super) fn guard_profile(
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
    lawful_factors.insert(LawfulFactorKind::RentalHistory, LawfulFactorValue::Count(0));
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

#[derive(Default, Clone)]
pub(super) struct MemoryRepository {
    pub(super) records: Arc<Mutex<HashMap<ApplicationId, ApplicationRecord>>>,
}

impl ApplicationRepository for MemoryRepository {
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
        guard.insert(record.profile.application_id.clone(), record);
        Ok(())
    }

    fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("repository mutex poisoned");
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
        self.events.lock().expect("alert mutex poisoned").clone()
    }
}

impl AlertPublisher for MemoryAlerts {
    fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError> {
        self.events
            .lock()
            .expect("alert mutex poisoned")
            .push(alert);
        Ok(())
    }
}

pub(super) struct ConflictRepository;

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

pub(super) struct UnavailableRepository;

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

pub(super) fn assert_conflict_response(response: Response) {
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

pub(super) async fn read_json_body(response: Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("json payload")
}

pub(super) fn evaluation_engine() -> EvaluationEngine {
    EvaluationEngine::new(evaluation_config())
}

pub(super) fn guard() -> ComplianceGuard {
    ComplianceGuard::default()
}

pub(super) fn manual_review_profile() -> ApplicationSubmission {
    let mut submission = submission();
    submission.criminal_history.push(CriminalRecord {
        classification: CriminalClassification::ViolentFelony,
        years_since: 2,
        jurisdiction: "Polk County".to_string(),
        description: "Assault".to_string(),
    });
    submission
}

pub(super) fn prohibited_submission() -> ApplicationSubmission {
    let mut submission = submission();
    submission.screening_answers.prohibited_preferences.push(
        ProhibitedScreeningPractice::ProtectedClassInquiry {
            field: "disability".to_string(),
        },
    );
    submission
}

pub(super) fn missing_income_submission() -> ApplicationSubmission {
    let mut submission = submission();
    submission.income.verified_income_sources.clear();
    submission
}

pub(super) fn application_router_with_service(
    service: VacancyApplicationService<MemoryRepository, MemoryAlerts>,
) -> axum::Router {
    application_router(Arc::new(service))
}
