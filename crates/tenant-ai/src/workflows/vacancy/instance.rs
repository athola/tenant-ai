use super::blueprint::VacancyWorkflowBlueprint;
use super::domain::{
    ComplianceNote, ComplianceSeverity, TaskStatus, TaskTemplate, VacancyError, VacancyRole,
    VacancyStage,
};
use super::report::{ComplianceAlert, TaskSnapshot, VacancyReport};
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TaskDetailView {
    pub key: &'static str,
    pub name: &'static str,
    pub stage: VacancyStage,
    pub stage_label: String,
    pub role: VacancyRole,
    pub role_label: String,
    pub due_date: NaiveDate,
    pub status: TaskStatus,
    pub status_label: String,
    pub completed_on: Option<NaiveDate>,
    pub deliverables: Vec<&'static str>,
    pub compliance: Vec<ComplianceNote>,
}

#[derive(Debug)]
pub struct VacancyWorkflowInstance {
    tasks: Vec<TaskInstance>,
}

#[derive(Debug, Clone)]
pub struct TaskInstance {
    pub template: TaskTemplate,
    pub due_date: NaiveDate,
    pub status: TaskStatus,
    pub completed_on: Option<NaiveDate>,
}

impl TaskInstance {
    pub fn to_view(&self) -> TaskDetailView {
        TaskDetailView {
            key: self.template.key,
            name: self.template.name,
            stage: self.template.stage,
            stage_label: self.template.stage.label().to_string(),
            role: self.template.primary_role,
            role_label: self.template.primary_role.label().to_string(),
            due_date: self.due_date,
            status: self.status,
            status_label: self.status.label().to_string(),
            completed_on: self.completed_on,
            deliverables: self.template.deliverables.clone(),
            compliance: self.template.compliance.clone(),
        }
    }
}

impl VacancyWorkflowInstance {
    pub fn new(
        blueprint: &VacancyWorkflowBlueprint,
        vacancy_start: NaiveDate,
        target_move_in: NaiveDate,
    ) -> Self {
        let tasks = blueprint
            .task_templates()
            .iter()
            .cloned()
            .map(|template| {
                let due_date = template.due.resolve(vacancy_start, target_move_in);
                TaskInstance {
                    template,
                    due_date,
                    status: TaskStatus::NotStarted,
                    completed_on: None,
                }
            })
            .collect();

        Self { tasks }
    }

    pub fn set_status(
        &mut self,
        task_key: &str,
        status: TaskStatus,
        completed_on: Option<NaiveDate>,
    ) -> Result<(), VacancyError> {
        let task = self
            .tasks
            .iter_mut()
            .find(|instance| instance.template.key == task_key)
            .ok_or_else(|| VacancyError::TaskNotFound(task_key.to_owned()))?;

        task.status = status;
        task.completed_on = match status {
            TaskStatus::Completed => completed_on,
            _ => None,
        };

        Ok(())
    }

    pub fn report(&self, today: NaiveDate) -> VacancyReport {
        let mut report = VacancyReport::default();

        for task in &self.tasks {
            let stage_entry = report
                .stage_progress
                .entry(task.template.stage)
                .or_default();
            stage_entry.total += 1;
            if task.status == TaskStatus::Completed {
                stage_entry.completed += 1;
            }

            let role_entry = report
                .role_load
                .entry(task.template.primary_role)
                .or_default();
            if task.status != TaskStatus::Completed {
                role_entry.open += 1;
                if task.due_date < today {
                    role_entry.overdue += 1;
                }
            }

            if task.status != TaskStatus::Completed && task.due_date < today {
                report.overdue_tasks.push(TaskSnapshot {
                    key: task.template.key,
                    name: task.template.name,
                    stage: task.template.stage,
                    role: task.template.primary_role,
                    due_date: task.due_date,
                    status: task.status,
                });

                for note in &task.template.compliance {
                    report.compliance_alerts.push(ComplianceAlert {
                        task_key: task.template.key,
                        topic: note.topic,
                        detail: note.detail,
                        severity: ComplianceSeverity::Critical,
                    });
                }
            } else if task.status != TaskStatus::Completed && !task.template.compliance.is_empty() {
                for note in &task.template.compliance {
                    report.compliance_alerts.push(ComplianceAlert {
                        task_key: task.template.key,
                        topic: note.topic,
                        detail: note.detail,
                        severity: ComplianceSeverity::Warning,
                    });
                }
            }
        }

        report
            .overdue_tasks
            .sort_by(|a, b| a.due_date.cmp(&b.due_date));

        report
    }

    pub fn tasks(&self) -> &[TaskInstance] {
        &self.tasks
    }

    pub fn task_details(&self) -> Vec<TaskDetailView> {
        let mut details: Vec<TaskDetailView> =
            self.tasks.iter().map(TaskInstance::to_view).collect();
        details.sort_by(|a, b| a.due_date.cmp(&b.due_date));
        details
    }
}
