mod config;
mod policy;
mod rules;

pub use config::EvaluationConfig;
pub use policy::{ApplicationDecision, DenialReason};

use super::domain::{ApplicantProfile, ApplicationId, LawfulFactorKind};
use policy::decide_outcome;
use serde::{Deserialize, Serialize};

/// Stateless evaluator that applies the rubric configuration to a profile.
pub struct EvaluationEngine {
    config: EvaluationConfig,
}

impl EvaluationEngine {
    pub fn new(config: EvaluationConfig) -> Self {
        Self { config }
    }

    pub fn score(&self, profile: &ApplicantProfile) -> EvaluationOutcome {
        let (components, total_score, signals) = rules::score_profile(profile, &self.config);

        let decision = decide_outcome(profile, &self.config, &signals);

        EvaluationOutcome {
            application_id: profile.application_id.clone(),
            decision,
            total_score,
            components,
        }
    }
}

/// Discrete contribution to an evaluation, allowing transparent audits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub factor: LawfulFactorKind,
    pub score: i16,
    pub notes: String,
}

/// Evaluation output describing the composite score and decision trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationOutcome {
    pub application_id: ApplicationId,
    pub decision: ApplicationDecision,
    pub total_score: i16,
    pub components: Vec<ScoreComponent>,
}
