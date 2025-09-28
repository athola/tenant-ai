use crate::workflows::vacancy::{
    TaskStatus, VacancyError, VacancyWorkflowBlueprint, VacancyWorkflowInstance,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

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

#[derive(Debug, Deserialize)]
struct ApolloRow {
    #[serde(rename = "Name")]
    name: String,
    #[serde(
        rename = "Completed At",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    completed_at: Option<String>,
    #[serde(
        rename = "Created At",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    created_at: Option<String>,
    #[serde(
        rename = "Last Modified",
        default,
        deserialize_with = "empty_string_as_none"
    )]
    last_modified: Option<String>,
}

impl ApolloRow {
    fn completed_date(&self) -> Option<NaiveDate> {
        self.completed_at
            .as_deref()
            .and_then(parse_datetime)
            .map(|dt| dt.date())
    }

    fn touched(&self) -> bool {
        match (
            self.created_at.as_deref().and_then(parse_datetime),
            self.last_modified.as_deref().and_then(parse_datetime),
        ) {
            (Some(created), Some(modified)) => modified > created,
            _ => false,
        }
    }
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|value| !value.trim().is_empty()))
}

fn parse_datetime(value: &str) -> Option<NaiveDateTime> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.naive_utc());
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return date.and_hms_opt(0, 0, 0);
    }

    None
}

fn normalize_name(value: &str) -> String {
    let cleaned = value.replace(['\u{feff}', '\u{200b}'], "");
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.to_ascii_lowercase()
}

fn apollo_name_map() -> HashMap<String, &'static str> {
    let mut map = HashMap::new();
    map.insert(
        normalize_name("Create and Publish Listing - Leasing Agent"),
        "marketing_publish_listing",
    );
    map.insert(
        normalize_name("Update Vacancy in AppFolio - Leasing Agent"),
        "marketing_update_appfolio",
    );
    map.insert(
        normalize_name("Manage Inquiries and Schedule Showings - Leasing Agent"),
        "screening_manage_inquiries",
    );
    map.insert(
        normalize_name("Process Rental Applications - Leasing Agent"),
        "screening_process_applications",
    );
    map.insert(
        normalize_name("Notify Applicants of Status - Leasing Agent"),
        "screening_notify_applicants",
    );
    map.insert(
        normalize_name("Prepare Lease Agreement - Leasing Agent"),
        "leasing_prepare_agreement",
    );
    map.insert(
        normalize_name("Collect Funds - Property Manager/Accounting"),
        "leasing_collect_funds",
    );
    map.insert(
        normalize_name("Conduct Move-In Inspection - Property Manager"),
        "leasing_conduct_move_in_inspection",
    );
    map.insert(
        normalize_name("Complete LIHTC Initial Certification - Compliance Coordinator"),
        "leasing_lihtc_certification",
    );
    map.insert(
        normalize_name("Start New Resident Workflow"),
        "handoff_start_new_resident_workflow",
    );
    map
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
        let mut csv_reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(reader);

        let blueprint = VacancyWorkflowBlueprint::standard();
        let mut instance = VacancyWorkflowInstance::new(&blueprint, vacancy_start, target_move_in);
        let mut applied: HashSet<&'static str> = HashSet::new();
        let name_map = apollo_name_map();

        for record in csv_reader.deserialize::<ApolloRow>() {
            let row = record?;
            let normalized_name = normalize_name(&row.name);
            let Some(&task_key) = name_map.get(&normalized_name) else {
                continue;
            };

            if applied.contains(task_key) {
                continue;
            }

            if let Some(completed_on) = row.completed_date() {
                instance.set_status(task_key, TaskStatus::Completed, Some(completed_on))?;
                applied.insert(task_key);
                continue;
            }

            if row.touched() {
                instance.set_status(task_key, TaskStatus::InProgress, None)?;
                applied.insert(task_key);
            }
        }

        Ok(instance)
    }
}
