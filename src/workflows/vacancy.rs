use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    fn resolve(&self, vacancy_start: NaiveDate, target_move_in: NaiveDate) -> NaiveDate {
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
pub struct VacancyWorkflowBlueprint {
    tasks: Vec<TaskTemplate>,
}

impl VacancyWorkflowBlueprint {
    pub fn standard() -> Self {
        Self {
            tasks: standard_task_templates(),
        }
    }

    pub fn tasks_for_stage(&self, stage: VacancyStage) -> Vec<&TaskTemplate> {
        self.tasks
            .iter()
            .filter(|task| task.stage == stage)
            .collect()
    }

    pub fn task_templates(&self) -> &[TaskTemplate] {
        &self.tasks
    }
}

fn standard_task_templates() -> Vec<TaskTemplate> {
    vec![
        TaskTemplate {
            key: "marketing_publish_listing",
            name: "Create and Publish Listing",
            stage: VacancyStage::MarketingAndAdvertising,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(0),
            deliverables: vec![
                "Draft a fresh listing that highlights unit features, affordability programs, and rent ready date.",
                "Upload current listing photos or virtual tour links before publishing.",
                "Syndicate to Zillow, Apartments.com, social media, and capture marketing URLs for reporting.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Iowa Code § 562A.29 reasonable re-rental efforts",
                detail: "Document every marketing channel touch to evidence reasonable efforts to re-rent (Iowa Code § 562A.29).",
            }],
        },
        TaskTemplate {
            key: "marketing_update_appfolio",
            name: "Update Vacancy Status in AppFolio",
            stage: VacancyStage::MarketingAndAdvertising,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(0),
            deliverables: vec![
                "Switch the unit status from \"Turnover\" to \"Vacant\" in AppFolio immediately after make-ready sign-off.",
                "Confirm listing syndication triggers fired for all partner channels.",
            ],
            compliance: vec![ComplianceNote {
                topic: "System of record accuracy",
                detail: "Accurate AppFolio statuses keep vacancy analytics, owner reporting, and marketing automation in sync.",
            }],
        },
        TaskTemplate {
            key: "screening_manage_inquiries",
            name: "Manage Inquiries and Schedule Showings",
            stage: VacancyStage::ScreeningAndApplication,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(0),
            deliverables: vec![
                "Respond to every inquiry within one business day using standardized messaging to preserve Fair Housing parity.",
                "Capture pre-screen answers covering move timeline, household composition, pets, and program eligibility.",
                "Offer pre-defined showing blocks via scheduling links to minimize back-and-forth.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Fair Housing and Iowa Civil Rights Act parity",
                detail: "Consistent response cadences prevent disparate treatment across protected classes and leave an audit trail.",
            }],
        },
        TaskTemplate {
            key: "screening_process_applications",
            name: "Process Rental Applications",
            stage: VacancyStage::ScreeningAndApplication,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(2),
            deliverables: vec![
                "Review each application within 48 hours and request missing fields immediately.",
                "Collect income, asset, and household documentation aligned with LIHTC and program requirements.",
                "Complete credit, background, and landlord verifications before rendering a decision.",
            ],
            compliance: vec![
                ComplianceNote {
                    topic: "Documented screening criteria",
                    detail: "Apply published screening criteria uniformly and retain documentation for adverse action defense.",
                },
                ComplianceNote {
                    topic: "LIHTC source-of-income verification",
                    detail: "Secure third-party income documentation to support Tenant Income Certification (TIC) files.",
                },
            ],
        },
        TaskTemplate {
            key: "screening_notify_applicants",
            name: "Notify Applicants of Status",
            stage: VacancyStage::ScreeningAndApplication,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(2),
            deliverables: vec![
                "Send approvals with next-step instructions and payment expectations.",
                "Issue denials with compliant adverse action language and timestamp outcomes in the CRM.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Adverse action documentation",
                detail: "Retain copies of denial notices and credit disclosures to satisfy Fair Credit Reporting Act obligations.",
            }],
        },
        TaskTemplate {
            key: "leasing_prepare_agreement",
            name: "Prepare Lease Agreement",
            stage: VacancyStage::LeaseSigningAndMoveIn,
            primary_role: VacancyRole::LeasingAgent,
            due: DueDateRule::DaysFromVacancy(5),
            deliverables: vec![
                "Merge approved terms into the LIHTC-compliant lease packet and distribute for e-signature.",
                "Confirm all addenda (e.g., VAWA, house rules) are attached before sending.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Lease artifact completeness",
                detail: "Incomplete lease packets jeopardize move-in readiness and downstream LIHTC audits.",
            }],
        },
        TaskTemplate {
            key: "leasing_collect_funds",
            name: "Collect Move-In Funds",
            stage: VacancyStage::LeaseSigningAndMoveIn,
            primary_role: VacancyRole::PropertyManagerAccounting,
            due: DueDateRule::DaysBeforeMoveIn(5),
            deliverables: vec![
                "Collect prorated rent, deposits, and fees; post receipts to the resident ledger.",
                "Confirm deposit amounts stay within Iowa caps (≤ two months rent).",
            ],
            compliance: vec![ComplianceNote {
                topic: "Security deposit limits",
                detail: "Deposits exceeding state limits expose the portfolio to statutory penalties.",
            }],
        },
        TaskTemplate {
            key: "leasing_conduct_move_in_inspection",
            name: "Conduct Move-In Inspection",
            stage: VacancyStage::LeaseSigningAndMoveIn,
            primary_role: VacancyRole::PropertyManager,
            due: DueDateRule::OnMoveIn,
            deliverables: vec![
                "Complete digital inspection checklist with tenant present and capture photos of every room.",
                "Upload signed inspection and media to AppFolio for permanent recordkeeping.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Move-in condition documentation",
                detail: "Thorough inspections limit security deposit disputes and support future turn charges.",
            }],
        },
        TaskTemplate {
            key: "leasing_lihtc_certification",
            name: "Complete LIHTC Initial Certification",
            stage: VacancyStage::LeaseSigningAndMoveIn,
            primary_role: VacancyRole::ComplianceCoordinator,
            due: DueDateRule::DaysBeforeMoveIn(3),
            deliverables: vec![
                "Collect signed Tenant Income Certification (TIC) and applicable student status affidavits.",
                "Verify income against current IFA limits and retain third-party documentation.",
                "Issue VAWA notices and ensure household files are audit ready.",
            ],
            compliance: vec![ComplianceNote {
                topic: "LIHTC eligibility lock-in",
                detail: "Certification must be finalized at least three days before move-in to maintain LIHTC compliance.",
            }],
        },
        TaskTemplate {
            key: "handoff_start_new_resident_workflow",
            name: "Handoff to New Resident Workflow",
            stage: VacancyStage::Handoff,
            primary_role: VacancyRole::PropertyManager,
            due: DueDateRule::OnMoveIn,
            deliverables: vec![
                "Update the unit status from \"Vacant\" to \"Occupied\" in AppFolio once keys are released.",
                "Trigger the New Resident onboarding workflow with welcome communications and follow-up tasks.",
            ],
            compliance: vec![ComplianceNote {
                topic: "Operational handoff completeness",
                detail: "Transitioning to onboarding ensures services, compliance tracking, and resident engagement continue seamlessly.",
            }],
        },
    ]
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
            stage_label: self.stage.label().to_string(),
            role: self.role,
            role_label: self.role.label().to_string(),
            due_date: self.due_date,
            status: self.status,
            status_label: self.status.label().to_string(),
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
            severity_label: self.severity.label().to_string(),
        }
    }
}

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
                        stage_label: stage.label().to_string(),
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
                    role_label: role.label().to_string(),
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

#[derive(Debug, Clone, Serialize)]
pub struct StageProgressEntry {
    pub stage: VacancyStage,
    pub stage_label: String,
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleLoadEntry {
    pub role: VacancyRole,
    pub role_label: String,
    pub open: usize,
    pub overdue: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSnapshotView {
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
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceAlertView {
    pub task_key: &'static str,
    pub topic: &'static str,
    pub detail: &'static str,
    pub severity: ComplianceSeverity,
    pub severity_label: String,
}

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

#[derive(Debug, Clone, Serialize)]
pub struct VacancyReportSummary {
    pub stage_progress: Vec<StageProgressEntry>,
    pub role_load: Vec<RoleLoadEntry>,
    pub overdue_tasks: Vec<TaskSnapshotView>,
    pub compliance_alerts: Vec<ComplianceAlertView>,
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

#[derive(Debug)]
pub enum VacancyError {
    TaskNotFound(String),
}

impl std::fmt::Display for VacancyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VacancyError::TaskNotFound(key) => write!(f, "task with key {} not found", key),
        }
    }
}

impl std::error::Error for VacancyError {}

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
