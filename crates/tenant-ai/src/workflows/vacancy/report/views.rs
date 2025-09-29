use super::super::domain::{ComplianceSeverity, TaskStatus, VacancyRole, VacancyStage};
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StageProgressEntry {
    pub stage: VacancyStage,
    pub stage_label: &'static str,
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleLoadEntry {
    pub role: VacancyRole,
    pub role_label: &'static str,
    pub open: usize,
    pub overdue: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSnapshotView {
    pub key: &'static str,
    pub name: &'static str,
    pub stage: VacancyStage,
    pub stage_label: &'static str,
    pub role: VacancyRole,
    pub role_label: &'static str,
    pub due_date: NaiveDate,
    pub status: TaskStatus,
    pub status_label: &'static str,
    pub completed_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceAlertView {
    pub task_key: &'static str,
    pub topic: &'static str,
    pub detail: &'static str,
    pub severity: ComplianceSeverity,
    pub severity_label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct VacancyReportSummary {
    pub stage_progress: Vec<StageProgressEntry>,
    pub role_load: Vec<RoleLoadEntry>,
    pub overdue_tasks: Vec<TaskSnapshotView>,
    pub compliance_alerts: Vec<ComplianceAlertView>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessLevel {
    OnTrack,
    Monitor,
    AtRisk,
}

impl ReadinessLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OnTrack => "On Track",
            Self::Monitor => "Monitor",
            Self::AtRisk => "At Risk",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VacancyInsights {
    pub readiness_score: u8,
    pub readiness_level: ReadinessLevel,
    pub expected_completion_pct: f32,
    pub days_until_move_in: i32,
    pub days_since_vacancy: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_stage: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_stage_completion: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_observations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation_triggers: Vec<String>,
}
