use super::super::domain::{
    ApplicantProfile, CriminalClassification, LawfulFactorKind, LawfulFactorValue,
};
use super::config::EvaluationConfig;
use super::rules::ScoreSignals;
use serde::{Deserialize, Serialize};

/// Adjudication outcome for a screened application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApplicationDecision {
    Approved,
    ConditionalApproval { required_actions: Vec<String> },
    Denied(DenialReason),
    ManualReview { reasons: Vec<String> },
}

impl ApplicationDecision {
    pub fn summary(&self) -> String {
        match self {
            ApplicationDecision::Approved => "application approved".to_string(),
            ApplicationDecision::ConditionalApproval { required_actions } => {
                if required_actions.is_empty() {
                    "conditional approval".to_string()
                } else {
                    format!("conditional approval: {}", required_actions.join(", "))
                }
            }
            ApplicationDecision::Denied(reason) => reason.summary(),
            ApplicationDecision::ManualReview { reasons } => {
                if reasons.is_empty() {
                    "requires manual review".to_string()
                } else {
                    format!("manual review required: {}", reasons.join("; "))
                }
            }
        }
    }
}

/// Enumerates lawful denial reasons to support adverse action notices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DenialReason {
    InsufficientIncome {
        required_ratio: f32,
        actual_ratio: f32,
    },
    AdverseCreditHistory,
    ExcessiveEvictions(u8),
    CriminalDisqualifier {
        classification: CriminalClassification,
        years_since: u8,
    },
    IncompleteDocumentation,
}

impl DenialReason {
    pub fn summary(&self) -> String {
        match self {
            DenialReason::InsufficientIncome {
                required_ratio,
                actual_ratio,
            } => format!(
                "denied for insufficient income (required {:.2}, actual {:.2})",
                required_ratio, actual_ratio
            ),
            DenialReason::AdverseCreditHistory => "denied for adverse credit history".to_string(),
            DenialReason::ExcessiveEvictions(count) => {
                format!("denied for {count} eviction(s)")
            }
            DenialReason::CriminalDisqualifier {
                classification,
                years_since,
            } => format!("denied for {:?} {} years ago", classification, years_since),
            DenialReason::IncompleteDocumentation => {
                "denied for incomplete documentation".to_string()
            }
        }
    }
}

pub(crate) fn decide_outcome(
    profile: &ApplicantProfile,
    config: &EvaluationConfig,
    signals: &ScoreSignals,
) -> ApplicationDecision {
    if let Some(detail) = &signals.violent_felony {
        return ApplicationDecision::ManualReview {
            reasons: vec![format!(
                "Recent violent felony within {} years: {}",
                config.violent_felony_lookback_years, detail
            )],
        };
    }

    if signals.rent_to_income > config.minimum_rent_to_income_ratio {
        return ApplicationDecision::Denied(DenialReason::InsufficientIncome {
            required_ratio: config.minimum_rent_to_income_ratio,
            actual_ratio: signals.rent_to_income,
        });
    }

    if let Some(min_credit) = config.minimum_credit_score {
        if signals
            .credit_score
            .map(|score| score < min_credit)
            .unwrap_or(true)
        {
            return ApplicationDecision::Denied(DenialReason::AdverseCreditHistory);
        }
    }

    if signals.eviction_count > config.max_evictions {
        return ApplicationDecision::Denied(DenialReason::ExcessiveEvictions(
            signals.eviction_count,
        ));
    }

    if profile
        .lawful_factors
        .get(&LawfulFactorKind::IowaSecurityDepositCompliance)
        == Some(&LawfulFactorValue::Boolean(false))
    {
        return ApplicationDecision::ConditionalApproval {
            required_actions: vec!["Adjust deposit to Iowa cap".to_string()],
        };
    }

    ApplicationDecision::Approved
}
