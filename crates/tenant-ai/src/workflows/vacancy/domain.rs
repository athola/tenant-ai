use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VacancyStage {
    MarketingAndAdvertising,
    ScreeningAndApplication,
    LeaseSigningAndMoveIn,
    Handoff,
}

impl VacancyStage {
    pub const fn ordered() -> [Self; 4] {
        [
            Self::MarketingAndAdvertising,
            Self::ScreeningAndApplication,
            Self::LeaseSigningAndMoveIn,
            Self::Handoff,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::MarketingAndAdvertising => "Marketing & Advertising",
            Self::ScreeningAndApplication => "Screening & Application",
            Self::LeaseSigningAndMoveIn => "Lease Signing & Move-In",
            Self::Handoff => "Handoff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VacancyRole {
    LeasingAgent,
    ComplianceCoordinator,
    PropertyManager,
    PropertyManagerAccounting,
}

impl VacancyRole {
    pub const fn ordered() -> [Self; 4] {
        [
            Self::LeasingAgent,
            Self::ComplianceCoordinator,
            Self::PropertyManager,
            Self::PropertyManagerAccounting,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::LeasingAgent => "Leasing Agent",
            Self::ComplianceCoordinator => "Compliance Coordinator",
            Self::PropertyManager => "Property Manager",
            Self::PropertyManagerAccounting => "Property Manager (Accounting)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Blocked,
}

impl TaskStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "Not Started",
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
            Self::Blocked => "Blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceSeverity {
    Warning,
    Critical,
}

impl ComplianceSeverity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DueDateRule {
    DaysFromVacancy(i64),
    DaysBeforeMoveIn(u32),
    OnMoveIn,
}

impl DueDateRule {
    pub(crate) fn resolve(&self, vacancy_start: NaiveDate, target_move_in: NaiveDate) -> NaiveDate {
        match self {
            DueDateRule::DaysFromVacancy(offset) => vacancy_start + Duration::days(*offset),
            DueDateRule::DaysBeforeMoveIn(days) => target_move_in - Duration::days(*days as i64),
            DueDateRule::OnMoveIn => target_move_in,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceNote {
    pub topic: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Clone)]
pub struct TaskTemplate {
    pub key: &'static str,
    pub name: &'static str,
    pub stage: VacancyStage,
    pub primary_role: VacancyRole,
    pub due: DueDateRule,
    pub deliverables: Vec<&'static str>,
    pub compliance: Vec<ComplianceNote>,
}

#[derive(Debug)]
pub enum VacancyError {
    TaskNotFound(String),
}

impl fmt::Display for VacancyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VacancyError::TaskNotFound(key) => write!(f, "task with key {} not found", key),
        }
    }
}

impl std::error::Error for VacancyError {}
