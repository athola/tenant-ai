use super::domain::{ComplianceNote, DueDateRule, TaskTemplate, VacancyRole, VacancyStage};

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
