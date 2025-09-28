use chrono::{Duration, NaiveDate};
use tenant_ai::workflows::apollo::ApolloVacancyImporter;
use tenant_ai::workflows::vacancy::{TaskStatus, VacancyWorkflowBlueprint};

fn sample_dates() -> (NaiveDate, NaiveDate) {
    let vacancy_start = NaiveDate::from_ymd_opt(2025, 9, 24).expect("valid vacancy start");
    let target_move_in = vacancy_start + Duration::days(14);
    (vacancy_start, target_move_in)
}

#[test]
fn importer_marks_completed_and_in_progress_tasks() {
    let csv = "Task ID,Created At,Completed At,Last Modified,Name\n\
1,2025-09-24T10:00:00Z,2025-09-25T12:15:00Z,2025-09-25T12:15:00Z,Create and Publish Listing - Leasing Agent\n\
2,2025-09-24T11:00:00Z,,2025-09-24T18:00:00Z,Update Vacancy in AppFolio - Leasing Agent\n";

    let (vacancy_start, target_move_in) = sample_dates();
    let instance =
        ApolloVacancyImporter::from_reader(csv.as_bytes(), vacancy_start, target_move_in)
            .expect("import succeeds");

    let listing_task = instance
        .tasks()
        .iter()
        .find(|task| task.template.key == "marketing_publish_listing")
        .expect("listing task present");
    assert_eq!(listing_task.status, TaskStatus::Completed);
    let completed_on = listing_task.completed_on.expect("completed date captured");
    assert_eq!(
        completed_on,
        NaiveDate::from_ymd_opt(2025, 9, 25).expect("valid completion date")
    );

    let appfolio_task = instance
        .tasks()
        .iter()
        .find(|task| task.template.key == "marketing_update_appfolio")
        .expect("appfolio update task present");
    assert_eq!(appfolio_task.status, TaskStatus::InProgress);
    assert!(appfolio_task.completed_on.is_none());
}

#[test]
fn importer_handles_full_apollo_export() {
    let data = include_bytes!("../Apollo_Apartments.csv");
    let (vacancy_start, target_move_in) = sample_dates();

    let instance = ApolloVacancyImporter::from_reader(&data[..], vacancy_start, target_move_in)
        .expect("apollo dataset imports");

    let blueprint = VacancyWorkflowBlueprint::standard();
    assert_eq!(instance.tasks().len(), blueprint.task_templates().len());
    assert!(instance.tasks().iter().all(|task| matches!(
        task.status,
        TaskStatus::NotStarted | TaskStatus::InProgress | TaskStatus::Completed
    )));
}
