use super::normalizer::normalize_name;
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Deserializer};
use std::io::Read;

#[derive(Debug)]
pub(crate) struct ApolloRecord {
    pub(crate) normalized_name: String,
    pub(crate) completed_on: Option<NaiveDate>,
    pub(crate) touched: bool,
}

pub(crate) fn parse_records<R: Read>(reader: R) -> Result<Vec<ApolloRecord>, csv::Error> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut records = Vec::new();

    for record in csv_reader.deserialize::<ApolloRow>() {
        let row = record?;
        let normalized_name = normalize_name(&row.name);
        let completed_on = row.completed_date();
        let touched = row.touched();

        records.push(ApolloRecord {
            normalized_name,
            completed_on,
            touched,
        });
    }

    Ok(records)
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
    D: Deserializer<'de>,
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

#[cfg(test)]
pub(crate) fn parse_datetime_for_tests(value: &str) -> Option<NaiveDateTime> {
    parse_datetime(value)
}
