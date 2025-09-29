use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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

/// High level status tracked throughout the vacancy application workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VacancyApplicationStatus {
    Submitted,
    UnderReview,
    Approved,
    Denied,
    Waitlisted,
}

impl VacancyApplicationStatus {
    pub const fn label(self) -> &'static str {
        match self {
            VacancyApplicationStatus::Submitted => "submitted",
            VacancyApplicationStatus::UnderReview => "under_review",
            VacancyApplicationStatus::Approved => "approved",
            VacancyApplicationStatus::Denied => "denied",
            VacancyApplicationStatus::Waitlisted => "waitlisted",
        }
    }
}
