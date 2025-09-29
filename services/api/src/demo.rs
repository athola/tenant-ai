use crate::infra::{
    default_evaluation_config, InMemoryAlertPublisher, InMemoryApplicationRepository,
};
use chrono::{Local, NaiveDate};
use clap::Args;
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
    } = args;

    let vacancy_start = vacancy_start.unwrap_or_else(|| Local::now().date_naive());
    let target_move_in =
        target_move_in.unwrap_or_else(|| vacancy_start + chrono::Duration::days(14));
    let today = today.unwrap_or_else(|| Local::now().date_naive());

    println!("Agentic workflow demo");
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
        include_tasks,
    );

    let lead_summary = synthetic_lead_automation_summary(today, target_move_in);
    println!(
        "\nCommunication automation snapshot (last {} days)",
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
        return Ok(());
    }

    println!("\nApplication intake demo (sensitive fields redacted)");
    let evaluation_config = default_evaluation_config();
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
        println!("  External alerts: none dispatched");
    } else {
        println!("  External alerts:");
        for alert in events {
            println!(
                "    - template={} -> {}",
                alert.template, alert.application_id.0
            );
        }
    }

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
    imported: bool,
    list_tasks: bool,
) {
    println!("Vacancy workflow demo");
    println!(
        "Vacancy window: {} -> {} (evaluated {})",
        vacancy_start, target_move_in, today
    );

    if imported {
        println!("Data source: Apollo CSV import");
    } else {
        println!("Data source: Standard blueprint (no Apollo data provided)");
    }

    let summary = report.summary();
    let insights = summary.insights(instance, vacancy_start, target_move_in, today);

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
