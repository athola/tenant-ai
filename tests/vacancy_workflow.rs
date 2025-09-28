use chrono::{Duration, NaiveDate};
use tenant_ai::workflows::vacancy::{
    ComplianceSeverity, TaskStatus, VacancyRole, VacancyStage, VacancyWorkflowBlueprint,
    VacancyWorkflowInstance,
};

fn vacancy_dates() -> (NaiveDate, NaiveDate) {
    let vacancy_start = NaiveDate::from_ymd_opt(2025, 9, 24).expect("valid vacancy start date");
    let target_move_in = vacancy_start + Duration::days(14);
    (vacancy_start, target_move_in)
}

#[test]
fn blueprint_captures_required_vacancy_structure() {
    let blueprint = VacancyWorkflowBlueprint::standard();

    let marketing_tasks = blueprint.tasks_for_stage(VacancyStage::MarketingAndAdvertising);
    assert_eq!(
        marketing_tasks.len(),
        2,
        "marketing stage should include publish and appfolio updates"
    );

    let publish_listing = marketing_tasks
        .iter()
        .find(|task| task.key == "marketing_publish_listing")
        .expect("publish listing task present");
    assert_eq!(publish_listing.primary_role, VacancyRole::LeasingAgent);
    assert!(publish_listing
        .deliverables
        .iter()
        .any(|step: &&str| step.contains("listing") && step.contains("photos")));
    assert!(publish_listing
        .compliance
        .iter()
        .any(|note| note.topic.contains("Iowa Code") && note.detail.contains("562A.29")));

    let screening_tasks = blueprint.tasks_for_stage(VacancyStage::ScreeningAndApplication);
    let manage_inquiries = screening_tasks
        .iter()
        .find(|task| task.key == "screening_manage_inquiries")
        .expect("manage inquiries task present");
    assert!(manage_inquiries
        .deliverables
        .iter()
        .any(|step: &&str| step.to_lowercase().contains("fair housing")));

    let lease_tasks = blueprint.tasks_for_stage(VacancyStage::LeaseSigningAndMoveIn);
    assert!(lease_tasks
        .iter()
        .any(|task| task.key == "leasing_lihtc_certification"));
}

#[test]
fn reporting_flags_overdue_and_compliance_gaps() {
    let blueprint = VacancyWorkflowBlueprint::standard();
    let (vacancy_start, target_move_in) = vacancy_dates();
    let mut instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);

    instance
        .set_status(
            "marketing_update_appfolio",
            TaskStatus::Completed,
            Some(vacancy_start),
        )
        .expect("able to mark task complete");
    instance
        .set_status(
            "screening_process_applications",
            TaskStatus::InProgress,
            None,
        )
        .expect("able to mark task in progress");

    let today = target_move_in - Duration::days(1);
    let report = instance.report(today);

    assert!(report
        .overdue_tasks
        .iter()
        .any(|task| task.key == "marketing_publish_listing"));

    assert!(report
        .compliance_alerts
        .iter()
        .any(|alert| alert.task_key == "marketing_publish_listing"
            && alert.topic.contains("Iowa Code")));

    assert!(report
        .compliance_alerts
        .iter()
        .any(|alert| alert.task_key == "leasing_lihtc_certification"
            && alert.severity == ComplianceSeverity::Critical));
}

#[test]
fn report_includes_stage_progress_and_role_load() {
    let blueprint = VacancyWorkflowBlueprint::standard();
    let (vacancy_start, target_move_in) = vacancy_dates();
    let mut instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);

    instance
        .set_status(
            "marketing_publish_listing",
            TaskStatus::Completed,
            Some(vacancy_start),
        )
        .expect("mark publish listing complete");
    instance
        .set_status(
            "marketing_update_appfolio",
            TaskStatus::Completed,
            Some(vacancy_start),
        )
        .expect("mark appfolio update complete");

    let report = instance.report(vacancy_start + Duration::days(1));

    let marketing_stage = report
        .stage_progress
        .get(&VacancyStage::MarketingAndAdvertising)
        .expect("marketing stage in report");
    assert_eq!(marketing_stage.completed, 2);
    assert_eq!(marketing_stage.total, 2);

    let leasing_role_load = report
        .role_load
        .get(&VacancyRole::LeasingAgent)
        .expect("leasing agent role load tracked");
    assert!(leasing_role_load.open >= 1);
}

#[test]
fn summary_produces_human_readable_views() {
    let blueprint = VacancyWorkflowBlueprint::standard();
    let (vacancy_start, target_move_in) = vacancy_dates();
    let instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);

    let summary = instance.report(vacancy_start).summary();

    assert_eq!(summary.stage_progress.len(), 4);
    assert_eq!(
        summary.stage_progress[0].stage,
        VacancyStage::MarketingAndAdvertising
    );
    assert_eq!(
        summary.stage_progress[0].stage_label,
        "Marketing & Advertising"
    );

    assert_eq!(summary.role_load.len(), 4);
    assert_eq!(summary.role_load[0].role, VacancyRole::LeasingAgent);
    assert_eq!(summary.role_load[0].role_label, "Leasing Agent");

    assert!(summary.overdue_tasks.is_empty());
    assert!(summary
        .compliance_alerts
        .iter()
        .any(|alert| alert.severity_label == "Warning"));
}
