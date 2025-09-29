use serde::{Deserialize, Serialize};

/// Rubric configuration describing the lawful scoring weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub minimum_rent_to_income_ratio: f32,
    pub minimum_credit_score: Option<u16>,
    pub max_evictions: u8,
    pub violent_felony_lookback_years: u8,
    pub non_violent_lookback_years: u8,
    pub misdemeanor_lookback_years: u8,
    pub deposit_cap_multiplier: f32,
}
