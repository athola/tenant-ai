use super::super::domain::{ComplianceSeverity, TaskStatus, VacancyRole, VacancyStage};
use super::super::instance::VacancyWorkflowInstance;
use super::views::{
    ComplianceAlertView, RoleLoadEntry, StageProgressEntry, TaskSnapshotView, VacancyInsights,
    VacancyReportSummary,
};
use chrono::NaiveDate;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct StageProgress {
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Default, Clone)]
pub struct RoleLoad {
    pub open: usize,
    pub overdue: usize,
}

#[derive(Debug, Default)]
pub struct VacancyReport {
    pub stage_progress: HashMap<VacancyStage, StageProgress>,
    pub role_load: HashMap<VacancyRole, RoleLoad>,
    pub overdue_tasks: Vec<TaskSnapshot>,
    pub compliance_alerts: Vec<ComplianceAlert>,
}

impl VacancyReport {
    pub fn summary(&self) -> VacancyReportSummary {
        let stage_progress = VacancyStage::ordered()
            .into_iter()
            .filter_map(|stage| {
                self.stage_progress
                    .get(&stage)
                    .map(|progress| StageProgressEntry {
                        stage,
                        stage_label: stage.label(),
                        completed: progress.completed,
                        total: progress.total,
                    })
            })
            .collect();

        let role_load = VacancyRole::ordered()
            .into_iter()
            .filter_map(|role| {
                self.role_load.get(&role).map(|load| RoleLoadEntry {
                    role,
                    role_label: role.label(),
                    open: load.open,
                    overdue: load.overdue,
                })
            })
            .collect();

        let overdue_tasks = self
            .overdue_tasks
            .iter()
            .map(TaskSnapshot::to_view)
            .collect();

        let compliance_alerts = self
            .compliance_alerts
            .iter()
            .map(ComplianceAlert::to_view)
            .collect();

        VacancyReportSummary {
            stage_progress,
            role_load,
            overdue_tasks,
            compliance_alerts,
        }
    }
}

impl VacancyReportSummary {
    pub fn insights(
        &self,
        instance: &VacancyWorkflowInstance,
        vacancy_start: NaiveDate,
        target_move_in: NaiveDate,
        today: NaiveDate,
    ) -> VacancyInsights {
        super::generate_insights(self, instance, vacancy_start, target_move_in, today)
    }
}

#[derive(Debug)]
pub struct TaskSnapshot {
    pub key: &'static str,
    pub name: &'static str,
    pub stage: VacancyStage,
    pub role: VacancyRole,
    pub due_date: NaiveDate,
    pub status: TaskStatus,
}

impl TaskSnapshot {
    pub fn to_view(&self) -> TaskSnapshotView {
        TaskSnapshotView {
            key: self.key,
            name: self.name,
            stage: self.stage,
            stage_label: self.stage.label(),
            role: self.role,
            role_label: self.role.label(),
            due_date: self.due_date,
            status: self.status,
            status_label: self.status.label(),
            completed_on: None,
        }
    }
}

#[derive(Debug)]
pub struct ComplianceAlert {
    pub task_key: &'static str,
    pub topic: &'static str,
    pub detail: &'static str,
    pub severity: ComplianceSeverity,
}

impl ComplianceAlert {
    pub fn to_view(&self) -> ComplianceAlertView {
        ComplianceAlertView {
            task_key: self.task_key,
            topic: self.topic,
            detail: self.detail,
            severity: self.severity,
            severity_label: self.severity.label(),
        }
    }
}
