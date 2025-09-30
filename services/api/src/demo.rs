use crate::infra::{
    default_evaluation_config, InMemoryAlertPublisher, InMemoryApplicationRepository,
};
use chrono::{Local, NaiveDate};
use clap::Args;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tenant_ai::error::AppError;
use tenant_ai::workflows::apollo::ApolloVacancyImporter;
use tenant_ai::workflows::vacancy::applications::{
    ApplicationRepository, ApplicationSubmission, CriminalClassification, CriminalRecord,
    DocumentCategory, DocumentDescriptor, EvaluationConfig, HouseholdComposition,
    IncomeDeclaration, LawfulFactorKind, LawfulFactorValue, RentalReference, ScreeningAnswers,
    SubsidyProgram, VacancyApplicationService, VacancyListingSnapshot,
};
use tenant_ai::workflows::vacancy::marketing::{
    DriveGateway, DriveMedia, DriveOperationError, ListingContext, MarketingInput,
    MarketingPublisher, ProspectCandidate,
};
use tenant_ai::workflows::vacancy::{
    VacancyReport, VacancyWorkflowBlueprint, VacancyWorkflowInstance,
};

#[derive(Args, Debug, Default)]
pub(crate) struct DemoArgs {
    /// Vacancy start date (YYYY-MM-DD). Defaults to today.
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) vacancy_start: Option<NaiveDate>,
    /// Target move-in date (YYYY-MM-DD). Defaults to vacancy_start + 14 days.
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) target_move_in: Option<NaiveDate>,
    /// Override the reporting date (defaults to today).
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) today: Option<NaiveDate>,
    /// Optional Apollo CSV export to hydrate the vacancy report.
    #[arg(long)]
    pub(crate) apollo_csv: Option<PathBuf>,
    /// Include a full task listing in the vacancy portion of the demo output.
    #[arg(long)]
    pub(crate) include_tasks: bool,
    /// Skip the application intake portion of the demo.
    #[arg(long)]
    pub(crate) skip_application: bool,
    /// Local directory to read unit photos from.
    #[arg(long)]
    pub(crate) photos_dir: Option<PathBuf>,
    /// Local directory to save the marketing report to.
    #[arg(long)]
    pub(crate) output_dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub(crate) struct VacancyReportArgs {
    /// Vacancy start date (YYYY-MM-DD)
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) vacancy_start: NaiveDate,
    /// Target move-in date (YYYY-MM-DD)
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) target_move_in: NaiveDate,
    /// Evaluation date for the report (defaults to today)
    #[arg(long, value_parser = crate::infra::parse_date)]
    pub(crate) today: Option<NaiveDate>,
    /// Optional Apollo CSV export to hydrate task progress
    #[arg(long)]
    pub(crate) apollo_csv: Option<PathBuf>,
    /// Include a full task listing in the output
    #[arg(long)]
    pub(crate) list_tasks: bool,
}

pub(crate) fn run_vacancy_report(args: VacancyReportArgs) -> Result<(), AppError> {
    let VacancyReportArgs {
        vacancy_start,
        target_move_in,
        today,
        apollo_csv,
        list_tasks,
    } = args;

    let today = today.unwrap_or_else(|| Local::now().date_naive());
    let (instance, imported) =
        load_vacancy_instance_from_path(apollo_csv, vacancy_start, target_move_in)?;

    let report = instance.report(today);
    render_vacancy_report(
        &instance,
        &report,
        vacancy_start,
        target_move_in,
        today,
        imported,
        list_tasks,
    );

    Ok(())
}

pub(crate) fn run_demo(args: DemoArgs) -> Result<(), AppError> {
    let DemoArgs {
        vacancy_start,
        target_move_in,
        today,
        apollo_csv,
        include_tasks,
        skip_application,
        photos_dir,
        output_dir,
    } = args;

    let vacancy_start = vacancy_start.unwrap_or_else(|| Local::now().date_naive());
    let target_move_in =
        target_move_in.unwrap_or_else(|| vacancy_start + chrono::Duration::days(14));
    let today = today.unwrap_or_else(|| Local::now().date_naive());
    let evaluation_config = default_evaluation_config();

    // --- Demo Header ---
    println!("=================================================");
    println!("      Agentic Property Orchestrator Demo");
    println!("=================================================");
    println!();

    // --- Configuration Summary ---
    println!("--- Demo Configuration ---");
    println!("Date of Report: {}", today);
    println!("Vacancy Window: {} -> {}", vacancy_start, target_move_in);
    if let Some(path) = &apollo_csv {
        println!("Data Source: Apollo CSV Import ({})", path.display());
    } else {
        println!("Data Source: Standard Blueprint (no Apollo data)");
    }
    if let Some(path) = &photos_dir {
        println!("Photo Source: Local Directory ({})", path.display());
    } else {
        println!("Photo Source: Sample Assets (no local directory provided)");
    }
    if let Some(path) = &output_dir {
        println!("Report Output: Local Directory ({})", path.display());
    } else {
        println!("Report Output: Console Only (no output directory provided)");
    }
    println!(
        "Include full task breakdown: {}",
        if include_tasks { "Yes" } else { "No" }
    );
    println!(
        "Skip application intake: {}",
        if skip_application { "Yes" } else { "No" }
    );
    println!();

    // --- Vacancy Workflow ---
    println!("--- 1. Vacancy Workflow Analysis ---");
    println!("Analyzing property vacancy to identify risks and opportunities.");
    println!("This section demonstrates automated task management and compliance monitoring.");
    println!();

    let (instance, imported) =
        load_vacancy_instance_from_path(apollo_csv.clone(), vacancy_start, target_move_in)?;
    let report = instance.report(today);
    render_vacancy_report(
        &instance,
        &report,
        vacancy_start,
        target_move_in,
        today,
        imported,
        include_tasks,
    );

    // --- Marketing ---
    println!();
    println!("--- 2. Automated Marketing ---");
    println!("Generating a marketing plan based on the vacancy and property data.");
    println!("This showcases dynamic content generation and compliance checks.");
    println!();
    render_marketing_listing_plan(target_move_in, &evaluation_config, &photos_dir, &output_dir);

    // --- Communication Automation ---
    println!();
    println!("--- 3. Communication Automation ---");
    println!("Simulating inbound lead management and automated responses.");
    println!("This highlights SLA adherence and efficient lead qualification.");
    let lead_summary = synthetic_lead_automation_summary(today, target_move_in);
    println!(
        "Communication automation snapshot (last {} days)",
        lead_summary.lookback_days
    );
    println!(
        "- {} inbound leads | {:.0}% automated first touch coverage",
        lead_summary.total_leads,
        lead_summary.automation_rate() * 100.0
    );
    println!(
        "- SLA target {} min | actual {:.1} min avg response | {:.0}% SLA adherence",
        lead_summary.sla_target_minutes,
        lead_summary.avg_first_response_minutes(),
        lead_summary.sla_met_pct() * 100.0
    );
    println!(
        "- {} conversations escalated to humans after automation",
        lead_summary.human_follow_up
    );
    println!("Channel mix:");
    for channel in &lead_summary.channels {
        println!(
            "  - {}: {} leads | {:.0}% automated | {:.1} min avg | {:.0}% SLA | {} live assist escalations",
            channel.channel,
            channel.leads,
            channel.automation_rate() * 100.0,
            channel.avg_first_response_minutes,
            channel.sla_met_pct * 100.0,
            channel.live_handoffs
        );
    }

    if skip_application {
        println!();
        println!("--- Demo Complete ---");
        println!("Skipped application intake as requested.");
        return Ok(());
    }

    // --- Application Intake ---
    println!();
    println!("--- 4. Application Intake & Evaluation ---");
    println!("Processing a sample rental application with automated evaluation.");
    println!("Demonstrates fair housing compliance and rapid decision-making.");
    println!("(Sensitive fields are redacted for privacy)");
    println!();

    let repository = Arc::new(InMemoryApplicationRepository::default());
    let alerts = Arc::new(InMemoryAlertPublisher::default());
    let service = Arc::new(VacancyApplicationService::new(
        repository.clone(),
        alerts.clone(),
        evaluation_config.clone(),
    ));

    let submission = demo_application_submission(target_move_in, &evaluation_config);
    let record = match service.submit(submission) {
        Ok(record) => record,
        Err(err) => {
            println!("  Submission rejected: {}", err);
            return Ok(());
        }
    };
    let public_view = record.status_view();
    println!(
        "- Received application {} -> status {}",
        public_view.application_id.0, public_view.status
    );
    println!("  Decision rationale: {}", public_view.decision_rationale);

    let outcome = match service.evaluate(&record.profile.application_id) {
        Ok(outcome) => outcome,
        Err(err) => {
            println!("  Evaluation unavailable: {}", err);
            return Ok(());
        }
    };
    println!(
        "  Evaluation decision: {} (score {})",
        outcome.decision.summary(),
        outcome.total_score
    );
    println!(
        "  Household summary: {} adults / {} children ({} bedrooms)",
        record.profile.household.adults,
        record.profile.household.children,
        record.profile.household.bedrooms_required
    );
    println!("  Score components (lawful factors only):");
    for component in &outcome.components {
        println!(
            "    - {:?}: {} ({})",
            component.factor, component.score, component.notes
        );
    }

    if let Some(LawfulFactorValue::Decimal(ratio)) = record
        .profile
        .lawful_factors
        .get(&LawfulFactorKind::RentToIncome)
    {
        println!("  Rent-to-income ratio (inputs redacted): {:.2}", ratio);
    }

    let stored_view = match repository.fetch(&record.profile.application_id) {
        Ok(Some(record)) => record.status_view(),
        Ok(None) => {
            println!("  Repository lookup returned no record");
            return Ok(());
        }
        Err(err) => {
            println!("  Repository unavailable: {}", err);
            return Ok(());
        }
    };
    match serde_json::to_string_pretty(&stored_view) {
        Ok(json) => println!("  Public status payload:\n{}", json),
        Err(err) => println!("  Public status payload unavailable: {}", err),
    }

    let events = alerts.events();
    if events.is_empty() {
        println!("\n  External alerts: none dispatched");
    } else {
        println!("\n  External alerts:");
        for alert in events {
            println!(
                "    - template={} -> {}",
                alert.template, alert.application_id.0
            );
        }
    }

    println!();
    println!("--- Demo Complete ---");

    Ok(())
}

fn demo_application_submission(
    target_move_in: NaiveDate,
    config: &EvaluationConfig,
) -> ApplicationSubmission {
    let listed_rent = 1180;
    let deposit_cap = ((listed_rent as f32) * config.deposit_cap_multiplier)
        .ceil()
        .clamp(0.0, u32::MAX as f32) as u32;
    let deposit_required = deposit_cap.saturating_sub(60);

    ApplicationSubmission {
        listing: VacancyListingSnapshot {
            unit_id: "A-201".to_string(),
            property_code: "APOLLO".to_string(),
            listed_rent,
            available_on: target_move_in,
            deposit_required,
        },
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
            requested_move_in: target_move_in,
            disclosed_vouchers: vec![SubsidyProgram {
                program: "HCV".to_string(),
                monthly_amount: 450,
            }],
            prohibited_preferences: Vec::new(),
        },
        income: IncomeDeclaration {
            gross_monthly_income: 4200,
            verified_income_sources: vec!["Employer verification".to_string()],
            housing_voucher_amount: Some(450),
        },
        rental_history: vec![RentalReference {
            property_name: "Riverfront Lofts".to_string(),
            paid_on_time: true,
            filed_eviction: false,
            tenancy_start: target_move_in
                .checked_sub_signed(chrono::Duration::days(365 * 2))
                .unwrap_or(target_move_in),
            tenancy_end: Some(target_move_in),
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
            storage_key: "redacted/sanitized".to_string(),
        }],
    }
}

pub(crate) fn load_vacancy_instance_from_path(
    apollo_csv: Option<PathBuf>,
    vacancy_start: NaiveDate,
    target_move_in: NaiveDate,
) -> Result<(VacancyWorkflowInstance, bool), AppError> {
    match apollo_csv {
        Some(path) => ApolloVacancyImporter::from_path(path, vacancy_start, target_move_in)
            .map(|instance| (instance, true))
            .map_err(AppError::from),
        None => {
            let blueprint = VacancyWorkflowBlueprint::standard();
            let instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);
            Ok((instance, false))
        }
    }
}

pub(crate) fn render_vacancy_report(
    instance: &VacancyWorkflowInstance,
    report: &VacancyReport,
    vacancy_start: NaiveDate,
    target_move_in: NaiveDate,
    today: NaiveDate,
    _imported: bool,
    list_tasks: bool,
) {
    let summary = report.summary();
    let insights = summary.insights(instance, vacancy_start, target_move_in, today);
    let total_completed: u32 = summary
        .stage_progress
        .iter()
        .map(|stage| stage.completed as u32)
        .sum();
    let total_tasks: u32 = summary
        .stage_progress
        .iter()
        .map(|stage| stage.total as u32)
        .sum();
    let overall_completion_pct = if total_tasks == 0 {
        0.0
    } else {
        total_completed as f32 / total_tasks as f32
    };

    println!("Executive briefing");
    println!(
        "- {} readiness ({}%) with {} overdue tasks and {} compliance alerts in scope.",
        insights.readiness_level.label(),
        insights.readiness_score,
        summary.overdue_tasks.len(),
        summary.compliance_alerts.len()
    );
    println!(
        "- Focus: {} | expected pace {:.0}% | actual completion {:.0}%.",
        insights.focus_stage.unwrap_or("no single stage flagged"),
        insights.expected_completion_pct * 100.0,
        overall_completion_pct * 100.0
    );
    if let Some(blocker) = insights.blockers.first() {
        println!("- Blocker to highlight: {}", blocker);
    } else {
        println!(
            "- Blocker to highlight: none flagged; emphasize how automation keeps the turn on schedule."
        );
    }
    if let Some(action) = insights.recommended_actions.first() {
        println!("- Priority follow-up: {}", action);
    }

    println!("\nWhy this matters for clients");
    println!(
        "- Readiness scoring quantifies schedule risk so asset managers can see automation's impact on turn timelines."
    );
    println!(
        "- Role workload and overdue surfacing explain why we assign automations by role—operators immediately see which teams benefit."
    );
    println!(
        "- Compliance alerts stay front and center to prove fair housing alignment during stakeholder Q&A."
    );
    println!(
        "- Automation triggers and AI observations double as implementation stories for onboarding and investor discussions."
    );

    println!("\nStage progress");
    for progress in &summary.stage_progress {
        println!(
            "- {}: {}/{} tasks completed",
            progress.stage_label, progress.completed, progress.total
        );
    }

    println!("\nRole workload");
    for load in &summary.role_load {
        println!(
            "- {}: {} open, {} overdue",
            load.role_label, load.open, load.overdue
        );
    }

    if summary.overdue_tasks.is_empty() {
        println!("\nOverdue tasks: none");
    } else {
        println!("\nOverdue tasks");
        for task in &summary.overdue_tasks {
            println!(
                "- {} ({}), role {}, due {}, status {}",
                task.name, task.stage_label, task.role_label, task.due_date, task.status_label
            );
        }
    }

    if summary.compliance_alerts.is_empty() {
        println!("\nCompliance alerts: none");
    } else {
        println!("\nCompliance alerts");
        for alert in &summary.compliance_alerts {
            println!(
                "- [{}] {}: {}",
                alert.severity_label, alert.topic, alert.detail
            );
        }
    }

    println!(
        "\nReadiness score: {}% ({})",
        insights.readiness_score,
        insights.readiness_level.label()
    );
    println!(
        "Expected pace {:.0}% | Days since vacancy {} | Days until move-in {}",
        insights.expected_completion_pct * 100.0,
        insights.days_since_vacancy,
        insights.days_until_move_in
    );

    if let Some(stage) = insights.focus_stage {
        if let Some(pct) = insights.focus_stage_completion {
            println!("Focus stage: {} ({:.0}% complete)", stage, pct * 100.0);
        } else {
            println!("Focus stage: {}", stage);
        }
    }

    if !insights.ai_observations.is_empty() {
        println!("\nAI observations");
        for note in &insights.ai_observations {
            println!("- {}", note);
        }
    }

    if !insights.recommended_actions.is_empty() {
        println!("\nRecommended actions");
        for action in &insights.recommended_actions {
            println!("- {}", action);
        }
    }

    if !insights.automation_triggers.is_empty() {
        println!("\nAutomation triggers");
        for trigger in &insights.automation_triggers {
            println!("- {}", trigger);
        }
    }

    if !insights.blockers.is_empty() {
        println!("\nTop blockers");
        for blocker in &insights.blockers {
            println!("- {}", blocker);
        }
    }

    if list_tasks {
        println!("\nTask breakdown by due date");
        for task in instance.task_details() {
            let completion_note = match task.completed_on {
                Some(date) => format!(" (completed {date})"),
                None => String::new(),
            };
            println!(
                "- {} | {} | {} | due {} | status {}{}",
                task.key,
                task.name,
                task.stage_label,
                task.due_date,
                task.status_label,
                completion_note
            );
        }
    }
}

fn render_marketing_listing_plan(
    target_move_in: NaiveDate,
    config: &EvaluationConfig,
    photos_dir: &Option<PathBuf>,
    output_dir: &Option<PathBuf>,
) {
    let listing = marketing_listing_context(target_move_in);

    let gateway: Box<dyn DriveGateway> = match (photos_dir, output_dir) {
        (Some(photos), Some(output)) => {
            println!("Marketing integration: local file system mode.");
            println!("  - Photos: {}", photos.display());
            println!("  - Output: {}", output.display());
            Box::new(LocalFileSystemGateway::new(photos.clone(), output.clone()))
        }
        _ => {
            println!("Marketing integration: offline demo mode.");
            println!("  Supply --photos-dir and --output-dir for local file integration.");
            Box::new(DemoDriveGateway)
        }
    };

    let publisher = MarketingPublisher::new(gateway, config.clone());

    let snapshot = VacancyListingSnapshot {
        unit_id: listing.unit_id.clone(),
        property_code: listing.property_code.clone(),
        listed_rent: listing.rent,
        available_on: listing.available_on,
        deposit_required: listing.deposit,
    };
    let input = MarketingInput {
        listing: listing.clone(),
        sample_applicants: marketing_sample_applicants(&snapshot),
    };

    match publisher.prepare_listing(input) {
        Ok(plan) => {
            println!("\nListing folder: {}", listing.drive_folder_id);
            println!("Generated document: {}", plan.google_doc_id);
            println!("Compliance summary: {}", plan.compliance_summary);

            if plan.missing_photos {
                println!("Media: requesting refreshed photos before publication.");
            } else {
                println!(
                    "Media: curated photo set ({} assets)",
                    plan.selected_photos.len()
                );
                for media in &plan.selected_photos {
                    match &media.web_view_link {
                        Some(link) => println!("  - {} -> {}", media.name, link),
                        None => println!("  - {}", media.name),
                    }
                }
            }

            println!("\nListing description preview:\n{}", plan.description);

            if plan.prospect_outcomes.is_empty() {
                println!("\nProspect evaluations: none provided");
            } else {
                println!("\nProspect evaluations:");
                for outcome in &plan.prospect_outcomes {
                    println!("- {} -> {}", outcome.name, outcome.decision);
                    println!("  {}", outcome.rationale);
                }
            }

            println!("\nMarketing narrative cues");
            println!(
                "- Local file sync keeps creative, compliance, and media updates in one place so leasing can publish within minutes."
            );
            println!(
                "- Automated copy with lawful-factor transparency is why we implemented the local file workflow instead of static templates."
            );
            println!(
                "- Prospect scenarios show how recommendations tie back to the evaluation engine customers license."
            );
        }
        Err(err) => {
            println!(
                "Marketing plan unavailable (file system integration offline): {}",
                err
            );
        }
    }
}

#[derive(Debug, Default, Clone)]
struct DemoDriveGateway;

impl DriveGateway for DemoDriveGateway {
    fn list_unit_media(&self, folder_id: &str) -> Result<Vec<DriveMedia>, DriveOperationError> {
        if folder_id != "demo-drive-folder" {
            return Ok(Vec::new());
        }

        Ok(vec![
            DriveMedia {
                file_id: "photo-1".to_string(),
                name: "living_room.jpg".to_string(),
                mime_type: Some("image/jpeg".to_string()),
                web_view_link: Some("https://drive.example/living_room".to_string()),
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
        ])
    }

    fn create_listing_document(
        &self,
        title: &str,
        _html_body: &str,
        _parent_folder_id: Option<&str>,
    ) -> Result<String, DriveOperationError> {
        let sanitized = title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        Ok(format!("demo-doc-{}", sanitized))
    }
}

#[derive(Debug, Clone)]
struct LocalFileSystemGateway {
    photos_dir: PathBuf,
    output_dir: PathBuf,
}

impl LocalFileSystemGateway {
    fn new(photos_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            photos_dir,
            output_dir,
        }
    }
}

impl DriveGateway for LocalFileSystemGateway {
    fn list_unit_media(&self, _folder_id: &str) -> Result<Vec<DriveMedia>, DriveOperationError> {
        let entries = fs::read_dir(&self.photos_dir).map_err(|e| {
            DriveOperationError::Backend(format!(
                "Failed to read photos directory '{}': {}",
                self.photos_dir.display(),
                e
            ))
        })?;

        let mut media = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                DriveOperationError::Backend(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            if path.is_file() {
                media.push(DriveMedia {
                    file_id: path.to_string_lossy().into_owned(),
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    mime_type: mime_guess::from_path(&path)
                        .first_raw()
                        .map(|s| s.to_string()),
                    web_view_link: Some(path.to_string_lossy().into_owned()),
                });
            }
        }
        Ok(media)
    }

    fn create_listing_document(
        &self,
        title: &str,
        html_body: &str,
        _parent_folder_id: Option<&str>,
    ) -> Result<String, DriveOperationError> {
        let sanitized_title = title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let file_name = format!("{}.html", sanitized_title);
        let output_path = self.output_dir.join(file_name);

        fs::write(&output_path, html_body).map_err(|e| {
            DriveOperationError::Backend(format!(
                "Failed to write report to '{}': {}",
                output_path.display(),
                e
            ))
        })?;

        Ok(output_path.to_string_lossy().into_owned())
    }
}

fn marketing_listing_context(target_move_in: NaiveDate) -> ListingContext {
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
            "Secure entry with intercom".to_string(),
            "Community fitness studio".to_string(),
        ],
        neighborhood_highlights: vec![
            "Two blocks from DART rapid bus".to_string(),
            "Adjacent to Riverwalk trail system".to_string(),
        ],
        nearby_schools: vec![
            "Downtown Elementary".to_string(),
            "Des Moines Central High".to_string(),
        ],
        drive_folder_id: "demo-drive-folder".to_string(),
        available_on: target_move_in,
    }
}

fn marketing_sample_applicants(snapshot: &VacancyListingSnapshot) -> Vec<ProspectCandidate> {
    vec![
        ProspectCandidate {
            name: "Voucher-supported household".to_string(),
            submission: marketing_submission_compliant(snapshot),
        },
        ProspectCandidate {
            name: "High-risk applicant".to_string(),
            submission: marketing_submission_high_risk(snapshot),
        },
    ]
}

fn marketing_submission_compliant(snapshot: &VacancyListingSnapshot) -> ApplicationSubmission {
    ApplicationSubmission {
        listing: snapshot.clone(),
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
            requested_move_in: snapshot.available_on,
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
            tenancy_start: snapshot
                .available_on
                .checked_sub_signed(chrono::Duration::days(365 * 2))
                .unwrap_or(snapshot.available_on),
            tenancy_end: Some(snapshot.available_on),
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
            storage_key: "drive://demo/income".to_string(),
        }],
    }
}

fn marketing_submission_high_risk(snapshot: &VacancyListingSnapshot) -> ApplicationSubmission {
    ApplicationSubmission {
        listing: snapshot.clone(),
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
            requested_move_in: snapshot.available_on,
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
            tenancy_start: snapshot
                .available_on
                .checked_sub_signed(chrono::Duration::days(365))
                .unwrap_or(snapshot.available_on),
            tenancy_end: Some(snapshot.available_on),
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
            storage_key: "drive://demo/pending".to_string(),
        }],
    }
}

#[derive(Debug)]
struct LeadChannelMetrics {
    channel: &'static str,
    leads: u32,
    automated_first_touch: u32,
    avg_first_response_minutes: f32,
    sla_met_pct: f32,
    live_handoffs: u32,
}

impl LeadChannelMetrics {
    fn automation_rate(&self) -> f32 {
        if self.leads == 0 {
            1.0
        } else {
            self.automated_first_touch as f32 / self.leads as f32
        }
    }
}

#[derive(Debug)]
struct LeadAutomationSummary {
    lookback_days: u32,
    total_leads: u32,
    automated_first_touch: u32,
    human_follow_up: u32,
    sla_target_minutes: u32,
    channels: Vec<LeadChannelMetrics>,
}

impl LeadAutomationSummary {
    fn automation_rate(&self) -> f32 {
        if self.total_leads == 0 {
            1.0
        } else {
            self.automated_first_touch as f32 / self.total_leads as f32
        }
    }

    fn avg_first_response_minutes(&self) -> f32 {
        if self.total_leads == 0 {
            return 0.0;
        }

        let weighted_total: f32 = self
            .channels
            .iter()
            .map(|channel| channel.avg_first_response_minutes * channel.leads as f32)
            .sum();
        weighted_total / self.total_leads as f32
    }

    fn sla_met_pct(&self) -> f32 {
        if self.total_leads == 0 {
            return 1.0;
        }

        let weighted_total: f32 = self
            .channels
            .iter()
            .map(|channel| channel.sla_met_pct * channel.leads as f32)
            .sum();
        weighted_total / self.total_leads as f32
    }
}

fn synthetic_lead_automation_summary(
    today: NaiveDate,
    target_move_in: NaiveDate,
) -> LeadAutomationSummary {
    let lookback_days = 7;
    let days_to_move_in = (target_move_in - today).num_days();
    let urgency = (14 - days_to_move_in).clamp(0, 10) as u32;
    let total_leads = 42 + urgency * 2;
    let mut automated_first_touch = ((total_leads as f32 * 0.84) + urgency as f32).round() as u32;
    if automated_first_touch > total_leads {
        automated_first_touch = total_leads;
    }

    let human_follow_up = total_leads.saturating_sub(automated_first_touch);
    let sla_target_minutes = 5;

    let mut sms_leads = ((total_leads as f32 * 0.45).round() as u32).max(1);
    if sms_leads > total_leads {
        sms_leads = total_leads;
    }

    let mut email_leads = ((total_leads as f32 * 0.35).round() as u32).max(1);
    if email_leads > total_leads.saturating_sub(sms_leads) {
        email_leads = total_leads.saturating_sub(sms_leads);
    }

    let phone_leads = total_leads.saturating_sub(sms_leads + email_leads);

    let sms_auto = (sms_leads as f32 * 0.92).round() as u32;
    let email_auto = (email_leads as f32 * 0.78).round() as u32;
    let phone_auto = automated_first_touch.saturating_sub(sms_auto + email_auto);

    let sms_channel = LeadChannelMetrics {
        channel: "SMS",
        leads: sms_leads,
        automated_first_touch: sms_auto.min(sms_leads),
        avg_first_response_minutes: 1.4,
        sla_met_pct: 0.97,
        live_handoffs: sms_leads.saturating_sub(sms_auto),
    };

    let email_channel = LeadChannelMetrics {
        channel: "Email",
        leads: email_leads,
        automated_first_touch: email_auto.min(email_leads),
        avg_first_response_minutes: 4.6,
        sla_met_pct: 0.88,
        live_handoffs: email_leads.saturating_sub(email_auto),
    };

    let phone_channel = LeadChannelMetrics {
        channel: "Voice",
        leads: phone_leads,
        automated_first_touch: phone_auto.min(phone_leads),
        avg_first_response_minutes: 2.9,
        sla_met_pct: 0.91,
        live_handoffs: phone_leads.saturating_sub(phone_auto),
    };

    LeadAutomationSummary {
        lookback_days,
        total_leads,
        automated_first_touch,
        human_follow_up,
        sla_target_minutes,
        channels: vec![sms_channel, email_channel, phone_channel],
    }
}
