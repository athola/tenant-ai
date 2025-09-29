use chrono::NaiveDate;
use metrics_exporter_prometheus::PrometheusHandle;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tenant_ai::workflows::vacancy::applications::{
    AlertError, AlertPublisher, AppFolioAlert, ApplicationId, ApplicationRecord,
    ApplicationRepository, EvaluationConfig, RepositoryError, VacancyApplicationStatus,
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) readiness: Arc<AtomicBool>,
    pub(crate) metrics: Arc<PrometheusHandle>,
}

#[derive(Default, Clone)]
pub(crate) struct InMemoryApplicationRepository {
    records: Arc<Mutex<HashMap<ApplicationId, ApplicationRecord>>>,
}

impl ApplicationRepository for InMemoryApplicationRepository {
    fn insert(&self, record: ApplicationRecord) -> Result<ApplicationRecord, RepositoryError> {
        let mut guard = self.records.lock().expect("repository mutex poisoned");
        if guard.contains_key(&record.profile.application_id) {
            return Err(RepositoryError::Conflict);
        }
        guard.insert(record.profile.application_id.clone(), record.clone());
        Ok(record)
    }

    fn update(&self, record: ApplicationRecord) -> Result<(), RepositoryError> {
        let mut guard = self.records.lock().expect("repository mutex poisoned");
        if guard.contains_key(&record.profile.application_id) {
            guard.insert(record.profile.application_id.clone(), record);
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    fn fetch(&self, id: &ApplicationId) -> Result<Option<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("repository mutex poisoned");
        Ok(guard.get(id).cloned())
    }

    fn pending(&self, _limit: usize) -> Result<Vec<ApplicationRecord>, RepositoryError> {
        let guard = self.records.lock().expect("repository mutex poisoned");
        Ok(guard
            .values()
            .filter(|record| record.status == VacancyApplicationStatus::UnderReview)
            .cloned()
            .collect())
    }
}

#[derive(Default, Clone)]
pub(crate) struct InMemoryAlertPublisher {
    events: Arc<Mutex<Vec<AppFolioAlert>>>,
}

impl AlertPublisher for InMemoryAlertPublisher {
    fn publish(&self, alert: AppFolioAlert) -> Result<(), AlertError> {
        let mut guard = self.events.lock().expect("alert mutex poisoned");
        guard.push(alert);
        Ok(())
    }
}

impl InMemoryAlertPublisher {
    pub(crate) fn events(&self) -> Vec<AppFolioAlert> {
        self.events.lock().expect("alert mutex poisoned").clone()
    }
}

pub(crate) fn default_evaluation_config() -> EvaluationConfig {
    EvaluationConfig {
        minimum_rent_to_income_ratio: 0.28,
        minimum_credit_score: Some(650),
        max_evictions: 0,
        violent_felony_lookback_years: 7,
        non_violent_lookback_years: 5,
        misdemeanor_lookback_years: 3,
        deposit_cap_multiplier: 2.0,
    }
}

pub(crate) fn parse_date(raw: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .map_err(|err| format!("failed to parse '{raw}' as YYYY-MM-DD ({err})"))
}

pub(crate) fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_date(&raw).map_err(serde::de::Error::custom)
}

pub(crate) fn deserialize_optional_date<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    opt.map(|value| parse_date(&value).map_err(serde::de::Error::custom))
        .transpose()
}
