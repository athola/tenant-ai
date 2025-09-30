use std::sync::Mutex;

use chrono::NaiveDate;
use tenant_ai::workflows::vacancy::applications::domain::{
    ApplicationSubmission, CriminalClassification, CriminalRecord, DocumentCategory,
    DocumentDescriptor, HouseholdComposition, IncomeDeclaration, RentalReference, ScreeningAnswers,
    SubsidyProgram, VacancyListingSnapshot,
};
use tenant_ai::workflows::vacancy::applications::EvaluationConfig;
use tenant_ai::workflows::vacancy::marketing::{
    DriveGateway, DriveMedia, DriveOperationError, ListingContext, MarketingInput, MarketingPlan,
    MarketingPublisher, ProspectCandidate,
};

struct FakeDriveGateway {
    media: Vec<DriveMedia>,
    created_docs: Mutex<Vec<(String, String)>>,
}

impl FakeDriveGateway {
    fn new(media: Vec<DriveMedia>) -> Self {
        Self {
            media,
            created_docs: Mutex::new(Vec::new()),
        }
    }
}

impl DriveGateway for FakeDriveGateway {
    fn list_unit_media(&self, _folder_id: &str) -> Result<Vec<DriveMedia>, DriveOperationError> {
        Ok(self.media.clone())
    }

    fn create_listing_document(
        &self,
        title: &str,
        html_body: &str,
        _parent_folder_id: Option<&str>,
    ) -> Result<String, DriveOperationError> {
        let mut guard = self.created_docs.lock().expect("doc mutex");
        guard.push((title.to_string(), html_body.to_string()));
        Ok("doc-123".to_string())
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

fn passing_submission(listing: &VacancyListingSnapshot) -> ApplicationSubmission {
    ApplicationSubmission {
        listing: listing.clone(),
        household: HouseholdComposition {
            adults: 2,
            children: 1,
            bedrooms_required: 2,
        },
        screening_answers: ScreeningAnswers {
            pets: false,
            service_animals: true,
            smoker: false,
            requested_accessibility_accommodations: vec!["Grab bars".to_string()],
            requested_move_in: listing.available_on,
            disclosed_vouchers: vec![SubsidyProgram {
                program: "HCV".to_string(),
                monthly_amount: 450,
            }],
            prohibited_preferences: Vec::new(),
        },
        income: IncomeDeclaration {
            gross_monthly_income: 4500,
            verified_income_sources: vec!["Employer verification".to_string()],
            housing_voucher_amount: Some(450),
        },
        rental_history: vec![RentalReference {
            property_name: "Riverfront Lofts".to_string(),
            paid_on_time: true,
            filed_eviction: false,
            tenancy_start: listing
                .available_on
                .checked_sub_signed(chrono::Duration::days(365 * 2))
                .unwrap(),
            tenancy_end: Some(listing.available_on),
        }],
        credit_score: Some(705),
        criminal_history: vec![CriminalRecord {
            classification: CriminalClassification::Misdemeanor,
            years_since: 6,
            jurisdiction: "Polk County".to_string(),
            description: "Expired registration".to_string(),
        }],
        supporting_documents: vec![DocumentDescriptor {
            name: "Income verification".to_string(),
            category: DocumentCategory::IncomeVerification,
            storage_key: "stored/in/cloud".to_string(),
        }],
    }
}

fn failing_submission(listing: &VacancyListingSnapshot) -> ApplicationSubmission {
    ApplicationSubmission {
        listing: listing.clone(),
        household: HouseholdComposition {
            adults: 1,
            children: 0,
            bedrooms_required: 1,
        },
        screening_answers: ScreeningAnswers {
            pets: true,
            service_animals: false,
            smoker: true,
            requested_accessibility_accommodations: Vec::new(),
            requested_move_in: listing.available_on,
            disclosed_vouchers: Vec::new(),
            prohibited_preferences: Vec::new(),
        },
        income: IncomeDeclaration {
            gross_monthly_income: 2100,
            verified_income_sources: vec!["Self reported".to_string()],
            housing_voucher_amount: None,
        },
        rental_history: vec![RentalReference {
            property_name: "Downtown Studios".to_string(),
            paid_on_time: false,
            filed_eviction: true,
            tenancy_start: listing
                .available_on
                .checked_sub_signed(chrono::Duration::days(365))
                .unwrap(),
            tenancy_end: Some(listing.available_on),
        }],
        credit_score: Some(520),
        criminal_history: vec![CriminalRecord {
            classification: CriminalClassification::NonViolentFelony,
            years_since: 2,
            jurisdiction: "Story County".to_string(),
            description: "Fraudulent check writing".to_string(),
        }],
        supporting_documents: vec![DocumentDescriptor {
            name: "Pay stub".to_string(),
            category: DocumentCategory::IncomeVerification,
            storage_key: "missing".to_string(),
        }],
    }
}

fn listing_context() -> ListingContext {
    let available_on = NaiveDate::from_ymd_opt(2025, 10, 1).expect("valid date");
    ListingContext {
        unit_id: "A-201".to_string(),
        property_code: "APOLLO".to_string(),
        property_name: "Apollo Apartments".to_string(),
        address: "123 Main St, Des Moines, IA".to_string(),
        bedrooms: 2,
        bathrooms: 1.5,
        square_feet: 940,
        rent: 1180,
        deposit: 2200,
        amenities: vec![
            "In-unit laundry".to_string(),
            "Secure entry".to_string(),
            "Bike storage".to_string(),
        ],
        neighborhood_highlights: vec![
            "Near Riverwalk trails".to_string(),
            "Farmers market access".to_string(),
        ],
        nearby_schools: vec![
            "Downtown Elementary".to_string(),
            "Des Moines Central High".to_string(),
        ],
        drive_folder_id: "drive-folder".to_string(),
        available_on,
    }
}

fn drive_media_samples() -> Vec<DriveMedia> {
    vec![
        DriveMedia {
            file_id: "photo-1".to_string(),
            name: "living_room.jpg".to_string(),
            mime_type: Some("image/jpeg".to_string()),
            web_view_link: Some("https://drive.example/living".to_string()),
        },
        DriveMedia {
            file_id: "photo-2".to_string(),
            name: "kitchen.jpg".to_string(),
            mime_type: Some("image/jpeg".to_string()),
            web_view_link: Some("https://drive.example/kitchen".to_string()),
        },
        DriveMedia {
            file_id: "photo-3".to_string(),
            name: "exterior.jpg".to_string(),
            mime_type: Some("image/jpeg".to_string()),
            web_view_link: Some("https://drive.example/exterior".to_string()),
        },
    ]
}

#[test]
fn marketing_plan_uses_existing_media_and_evaluations() {
    let context = listing_context();
    let listing_snapshot = VacancyListingSnapshot {
        unit_id: context.unit_id.clone(),
        property_code: context.property_code.clone(),
        listed_rent: context.rent,
        available_on: context.available_on,
        deposit_required: context.deposit,
    };

    let input = MarketingInput {
        listing: context,
        sample_applicants: vec![
            ProspectCandidate {
                name: "Voucher-supported household".to_string(),
                submission: passing_submission(&listing_snapshot),
            },
            ProspectCandidate {
                name: "High-risk applicant".to_string(),
                submission: failing_submission(&listing_snapshot),
            },
        ],
    };

    let drive = FakeDriveGateway::new(drive_media_samples());
    let config = evaluation_config();
    let publisher = MarketingPublisher::new(drive, config.clone());

    let plan: MarketingPlan = publisher
        .prepare_listing(input)
        .expect("marketing plan should be generated");

    assert_eq!(plan.selected_photos.len(), 3);
    assert!(!plan.missing_photos);
    assert_eq!(plan.google_doc_id, "doc-123");
    assert!(plan.description.contains("Apollo Apartments"));
    assert!(plan.description.contains("No smoking"));
    assert!(plan.description.contains("Service animals"));
    assert!(plan.description.contains("Fair Housing Act"));
    assert!(plan.compliance_summary.contains("Fair Housing"));
    assert!(plan.compliance_summary.contains("Iowa Civil Rights"));

    let required_income_copy = format!(
        "{:.1}x rent",
        1.0 / f64::from(config.minimum_rent_to_income_ratio)
    );
    assert!(
        plan.description.contains(&required_income_copy),
        "expected marketing copy to reference {required_income_copy}",
    );

    let approvals: Vec<_> = plan
        .prospect_outcomes
        .iter()
        .filter(|outcome| outcome.decision.to_lowercase().contains("approved"))
        .collect();
    assert!(!approvals.is_empty(), "expected at least one approval");

    let denials: Vec<_> = plan
        .prospect_outcomes
        .iter()
        .filter(|outcome| outcome.decision.to_lowercase().contains("denied"))
        .collect();
    assert!(!denials.is_empty(), "expected at least one denial");

    assert!(plan
        .prospect_outcomes
        .iter()
        .any(|outcome| outcome.rationale.to_lowercase().contains("criminal")));
}

#[test]
fn marketing_plan_requests_new_photos_when_library_empty() {
    let context = listing_context();
    let listing_snapshot = VacancyListingSnapshot {
        unit_id: context.unit_id.clone(),
        property_code: context.property_code.clone(),
        listed_rent: context.rent,
        available_on: context.available_on,
        deposit_required: context.deposit,
    };

    let input = MarketingInput {
        listing: context,
        sample_applicants: vec![ProspectCandidate {
            name: "Voucher-supported household".to_string(),
            submission: passing_submission(&listing_snapshot),
        }],
    };

    let drive = FakeDriveGateway::new(Vec::new());
    let publisher = MarketingPublisher::new(drive, evaluation_config());

    let plan = publisher
        .prepare_listing(input)
        .expect("marketing plan even without photos");

    assert!(plan.selected_photos.is_empty());
    assert!(plan.missing_photos);
    assert!(plan
        .description
        .to_lowercase()
        .contains("requesting refreshed photography"));
}
