use std::collections::BTreeMap;

use super::domain::{
    ApplicantProfile, ApplicationId, ApplicationSubmission, LawfulFactorKind, LawfulFactorValue,
    ProhibitedScreeningPractice,
};
use super::evaluation::EvaluationConfig;

/// Validation errors raised by the compliance guard.
#[derive(Debug, thiserror::Error)]
pub enum ComplianceViolation {
    #[error("submission captured prohibited screening practice: {0:?}")]
    ProhibitedPractice(ProhibitedScreeningPractice),
    #[error("security deposit exceeds Iowa two month cap (required <= {max:?}, found {found:?})")]
    IowaSecurityDepositCap { max: u32, found: u32 },
    #[error("missing verified income documentation for LIHTC/IFA requirements")]
    MissingIncomeDocumentation,
    #[error("household composition incomplete")]
    IncompleteHousehold,
}

const DEFAULT_DEPOSIT_CAP_MULTIPLIER: f32 = 2.0;

/// Policy dial backing compliance validation (e.g., Iowa security-deposit rules).
#[derive(Debug, Clone)]
pub struct CompliancePolicy {
    deposit_cap_multiplier: f32,
}

impl CompliancePolicy {
    pub fn new(deposit_cap_multiplier: f32) -> Self {
        let sanitized = if deposit_cap_multiplier.is_finite() && deposit_cap_multiplier > 0.0 {
            deposit_cap_multiplier
        } else {
            DEFAULT_DEPOSIT_CAP_MULTIPLIER
        };

        Self {
            deposit_cap_multiplier: sanitized,
        }
    }

    pub fn deposit_cap_multiplier(&self) -> f32 {
        self.deposit_cap_multiplier
    }

    pub fn max_deposit_for(&self, listed_rent: u32) -> u32 {
        if listed_rent == 0 {
            return 0;
        }

        let max = (listed_rent as f64) * (self.deposit_cap_multiplier as f64);
        let bounded = max.ceil().min(u32::MAX as f64);
        bounded as u32
    }
}

impl Default for CompliancePolicy {
    fn default() -> Self {
        Self::new(DEFAULT_DEPOSIT_CAP_MULTIPLIER)
    }
}

impl From<&EvaluationConfig> for CompliancePolicy {
    fn from(config: &EvaluationConfig) -> Self {
        Self::new(config.deposit_cap_multiplier)
    }
}

/// Guard responsible for producing `ApplicantProfile` instances.
#[derive(Debug, Clone)]
pub struct ComplianceGuard {
    policy: CompliancePolicy,
}

impl Default for ComplianceGuard {
    fn default() -> Self {
        Self::with_policy(CompliancePolicy::default())
    }
}

impl ComplianceGuard {
    pub fn with_policy(policy: CompliancePolicy) -> Self {
        Self { policy }
    }

    pub fn from_config(config: &EvaluationConfig) -> Self {
        Self::with_policy(CompliancePolicy::from(config))
    }

    pub fn policy(&self) -> &CompliancePolicy {
        &self.policy
    }

    /// Convert an inbound submission into a sanitized applicant profile.
    pub fn profile_from_submission(
        &self,
        submission: ApplicationSubmission,
    ) -> Result<ApplicantProfile, ComplianceViolation> {
        if let Some(prohibited) = submission
            .screening_answers
            .prohibited_preferences
            .first()
            .cloned()
        {
            return Err(ComplianceViolation::ProhibitedPractice(prohibited));
        }

        if submission.income.verified_income_sources.is_empty() {
            return Err(ComplianceViolation::MissingIncomeDocumentation);
        }

        let household = submission.household;
        if household.adults == 0 && household.children == 0 {
            return Err(ComplianceViolation::IncompleteHousehold);
        }

        let deposit_cap = self.policy.max_deposit_for(submission.listing.listed_rent);
        if submission.listing.deposit_required > deposit_cap {
            return Err(ComplianceViolation::IowaSecurityDepositCap {
                max: deposit_cap,
                found: submission.listing.deposit_required,
            });
        }

        let mut lawful_factors = BTreeMap::new();

        if submission.income.gross_monthly_income == 0 {
            return Err(ComplianceViolation::MissingIncomeDocumentation);
        }

        let rent_to_income =
            submission.listing.listed_rent as f32 / submission.income.gross_monthly_income as f32;
        lawful_factors.insert(
            LawfulFactorKind::RentToIncome,
            LawfulFactorValue::Decimal(rent_to_income),
        );

        if let Some(score) = submission.credit_score {
            lawful_factors.insert(
                LawfulFactorKind::CreditScore,
                LawfulFactorValue::Count(score as u32),
            );
        }

        let eviction_count = submission
            .rental_history
            .iter()
            .filter(|reference| reference.filed_eviction)
            .count() as u32;
        lawful_factors.insert(
            LawfulFactorKind::RentalHistory,
            LawfulFactorValue::Count(eviction_count),
        );

        if !submission.criminal_history.is_empty() {
            let window = submission
                .criminal_history
                .iter()
                .map(|record| record.years_since as f32)
                .fold(f32::INFINITY, f32::min);
            lawful_factors.insert(
                LawfulFactorKind::CriminalHistoryWindow,
                LawfulFactorValue::Decimal(if window.is_finite() { window } else { 0.0 }),
            );
        }

        let voucher_coverage = submission
            .income
            .housing_voucher_amount
            .map(|amount| amount as f32 / submission.listing.listed_rent as f32)
            .unwrap_or(0.0);
        lawful_factors.insert(
            LawfulFactorKind::VoucherCoverage,
            LawfulFactorValue::Decimal(voucher_coverage),
        );

        lawful_factors.insert(
            LawfulFactorKind::IowaSecurityDepositCompliance,
            LawfulFactorValue::Boolean(true),
        );

        Ok(ApplicantProfile {
            application_id: ApplicationId("pending".to_string()),
            lawful_factors,
            household,
            listing: submission.listing,
            declared_income: submission.income,
            rental_history: submission.rental_history,
            credit_score: submission.credit_score,
            criminal_history: submission.criminal_history,
            accommodations: submission
                .screening_answers
                .requested_accessibility_accommodations,
        })
    }
}
