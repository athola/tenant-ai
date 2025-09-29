mod mapping;
mod normalizer;
mod parser;

use crate::workflows::vacancy::{
    domain::{TaskStatus, VacancyError},
    VacancyWorkflowBlueprint, VacancyWorkflowInstance,
};
use chrono::NaiveDate;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use parser::ApolloRecord;

#[derive(Debug)]
pub enum ApolloVacancyImportError {
    Io(std::io::Error),
    Csv(csv::Error),
    Vacancy(VacancyError),
}

impl std::fmt::Display for ApolloVacancyImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApolloVacancyImportError::Io(err) => write!(f, "failed to read Apollo export: {}", err),
            ApolloVacancyImportError::Csv(err) => write!(f, "invalid Apollo CSV data: {}", err),
            ApolloVacancyImportError::Vacancy(err) => write!(
                f,
                "could not apply Apollo data to vacancy workflow: {}",
                err
            ),
        }
    }
}

impl std::error::Error for ApolloVacancyImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApolloVacancyImportError::Io(err) => Some(err),
            ApolloVacancyImportError::Csv(err) => Some(err),
            ApolloVacancyImportError::Vacancy(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ApolloVacancyImportError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<csv::Error> for ApolloVacancyImportError {
    fn from(err: csv::Error) -> Self {
        Self::Csv(err)
    }
}

impl From<VacancyError> for ApolloVacancyImportError {
    fn from(err: VacancyError) -> Self {
        Self::Vacancy(err)
    }
}

pub struct ApolloVacancyImporter;

impl ApolloVacancyImporter {
    pub fn from_path<P: AsRef<Path>>(
        path: P,
        vacancy_start: NaiveDate,
        target_move_in: NaiveDate,
    ) -> Result<VacancyWorkflowInstance, ApolloVacancyImportError> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(file, vacancy_start, target_move_in)
    }

    pub fn from_reader<R: Read>(
        reader: R,
        vacancy_start: NaiveDate,
        target_move_in: NaiveDate,
    ) -> Result<VacancyWorkflowInstance, ApolloVacancyImportError> {
        let blueprint = VacancyWorkflowBlueprint::standard();
        let mut instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);
        let mut applied: HashSet<&'static str> = HashSet::new();

        for record in parser::parse_records(reader)? {
            if let Some(task_key) = mapping::task_key_for_normalized(&record.normalized_name) {
                if applied.contains(task_key) {
                    continue;
                }

                apply_record(task_key, record, &mut instance, &mut applied)?;
            }
        }

        Ok(instance)
    }
}

fn apply_record(
    task_key: &'static str,
    record: ApolloRecord,
    instance: &mut VacancyWorkflowInstance,
    applied: &mut HashSet<&'static str>,
) -> Result<(), VacancyError> {
    if let Some(completed_on) = record.completed_on {
        instance.set_status(task_key, TaskStatus::Completed, Some(completed_on))?;
        applied.insert(task_key);
    } else if record.touched {
        instance.set_status(task_key, TaskStatus::InProgress, None)?;
        applied.insert(task_key);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn vacancy_dates() -> (NaiveDate, NaiveDate) {
        let vacancy_start = NaiveDate::from_ymd_opt(2025, 9, 24).expect("valid start");
        let move_in = vacancy_start + chrono::Duration::days(14);
        (vacancy_start, move_in)
    }

    #[test]
    fn parse_datetime_supports_rfc3339_and_date_strings() {
        let rfc = parser::parse_datetime_for_tests("2025-09-24T10:00:00Z").expect("parse rfc");
        assert_eq!(
            rfc,
            NaiveDate::from_ymd_opt(2025, 9, 24)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap()
        );

        let date = parser::parse_datetime_for_tests("2025-09-30").expect("parse date");
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(2025, 9, 30)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );

        assert!(parser::parse_datetime_for_tests("  ").is_none());
        assert!(parser::parse_datetime_for_tests("not-a-date").is_none());
    }

    #[test]
    fn normalize_name_removes_whitespace_and_case() {
        let source = "\u{feff}Create  and  Publish  Listing  -  Leasing  Agent";
        let normalized = normalizer::normalize_for_tests(source);
        assert_eq!(normalized, "create and publish listing - leasing agent");
    }

    #[test]
    fn apollo_row_detects_completion_and_touch() {
        let record = parser::parse_records(Cursor::new(
            "Name,Completed At,Created At,Last Modified\nTask,2025-09-25T12:15:00Z,2025-09-24T10:00:00Z,2025-09-24T12:00:00Z\n",
        ))
        .expect("parse")
        .pop()
        .expect("record");
        assert_eq!(
            record.completed_on.expect("completed"),
            NaiveDate::from_ymd_opt(2025, 9, 25).unwrap()
        );
        assert!(record.touched);

        let record = parser::parse_records(Cursor::new(
            "Name,Completed At,Created At,Last Modified\nTask,,,\n",
        ))
        .expect("parse")
        .pop()
        .expect("record");
        assert!(!record.touched);
    }

    #[test]
    fn importer_handles_duplicate_rows_without_overwriting() {
        let csv = "Name,Created At,Completed At,Last Modified\n\
Create and Publish Listing - Leasing Agent,2025-09-24T10:00:00Z,2025-09-25T12:00:00Z,2025-09-25T12:00:00Z\n\
Create and Publish Listing - Leasing Agent,2025-09-24T11:00:00Z,,2025-09-24T12:30:00Z\n";
        let (vacancy_start, move_in) = vacancy_dates();
        let instance = ApolloVacancyImporter::from_reader(Cursor::new(csv), vacancy_start, move_in)
            .expect("import succeeds");

        let publish_listing = instance
            .tasks()
            .iter()
            .find(|task| task.template.key == "marketing_publish_listing")
            .expect("task present");
        assert_eq!(publish_listing.status, TaskStatus::Completed);
    }

    #[test]
    fn importer_ignores_unknown_task_names() {
        let csv = "Name,Created At,Completed At,Last Modified\nUnknown Task,2025-09-24T10:00:00Z,,2025-09-24T12:00:00Z\n";
        let (vacancy_start, move_in) = vacancy_dates();
        let instance = ApolloVacancyImporter::from_reader(Cursor::new(csv), vacancy_start, move_in)
            .expect("import succeeds");

        assert!(instance
            .tasks()
            .iter()
            .all(|task| task.status == TaskStatus::NotStarted));
    }

    #[test]
    fn importer_from_path_propagates_io_errors() {
        let (vacancy_start, move_in) = vacancy_dates();
        let error =
            ApolloVacancyImporter::from_path("./does-not-exist.csv", vacancy_start, move_in)
                .expect_err("expected io error");

        match error {
            ApolloVacancyImportError::Io(_) => {}
            other => panic!("expected io error, got {other:?}"),
        }
    }

    #[test]
    fn mapping_recognizes_known_tasks() {
        assert_eq!(
            mapping::lookup_for_tests("Create and Publish Listing - Leasing Agent"),
            Some("marketing_publish_listing")
        );
        assert_eq!(
            mapping::lookup_for_tests("Manage Inquiries & Schedule Showings - Leasing Agent"),
            Some("screening_manage_inquiries")
        );
        assert_eq!(
            mapping::lookup_for_tests("Collect Funds - Property Manager / Accounting"),
            Some("leasing_collect_funds")
        );
        assert_eq!(
            mapping::lookup_for_tests("Hand Over Keys & Welcome Tenant - Leasing Agent"),
            Some("handoff_start_new_resident_workflow")
        );
    }
}
