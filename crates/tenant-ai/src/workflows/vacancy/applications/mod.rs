//! Vacancy application intake, evaluation, and compliance scaffolding.
//!
//! The concrete implementations are intentionally left as `todo!()` placeholders so that
//! the accompanying tests can drive out the full behavior using a TDD workflow. The types
//! and signatures defined here represent the initial contract that the new vacancy intake
//! pipeline will satisfy once implemented.

pub(crate) mod compliance;
pub mod domain;
pub(crate) mod evaluation;
pub mod repository;
pub mod router;
pub mod service;

#[cfg(test)]
mod tests;

pub use domain::{
    ApplicantProfile, ApplicationId, ApplicationSubmission, CriminalClassification, CriminalRecord,
    DocumentCategory, DocumentDescriptor, HouseholdComposition, IncomeDeclaration,
    LawfulFactorKind, LawfulFactorValue, ProhibitedScreeningPractice, RentalReference,
    ScreeningAnswers, SubsidyProgram, VacancyApplicationStatus, VacancyListingSnapshot,
};
pub use evaluation::{ApplicationDecision, DenialReason, EvaluationConfig, EvaluationOutcome};
pub use repository::{
    AlertError, AlertPublisher, AppFolioAlert, ApplicationRecord, ApplicationRepository,
    ApplicationStatusView, RepositoryError,
};
pub use router::application_router;
pub use service::{ApplicationServiceError, VacancyApplicationService};
