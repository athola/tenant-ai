mod insights;
mod summary;
pub mod views;

pub use summary::VacancyReport;

pub(crate) use insights::generate_insights;
pub(crate) use summary::{ComplianceAlert, TaskSnapshot};
