use super::super::domain::{
    ApplicantProfile, CriminalClassification, LawfulFactorKind, LawfulFactorValue,
};
use super::config::EvaluationConfig;
use super::ScoreComponent;

pub(crate) struct ScoreSignals {
    pub rent_to_income: f32,
    pub credit_score: Option<u16>,
    pub eviction_count: u8,
    pub violent_felony: Option<String>,
}

pub(crate) fn score_profile(
    profile: &ApplicantProfile,
    config: &EvaluationConfig,
) -> (Vec<ScoreComponent>, i16, ScoreSignals) {
    let mut components = Vec::new();
    let mut total_score: i16 = 0;

    let rent_to_income = profile
        .lawful_factors
        .get(&LawfulFactorKind::RentToIncome)
        .and_then(|value| match value {
            LawfulFactorValue::Decimal(ratio) => Some(*ratio),
            _ => None,
        })
        .unwrap_or_else(|| {
            profile.listing.listed_rent as f32 / profile.declared_income.gross_monthly_income as f32
        });

    let rent_ratio_within = rent_to_income <= config.minimum_rent_to_income_ratio;
    if rent_ratio_within {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::RentToIncome,
            score: 30,
            notes: format!(
                "rent-to-income ratio {:.2} within policy threshold {:.2}",
                rent_to_income, config.minimum_rent_to_income_ratio
            ),
        });
        total_score += 30;
    } else {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::RentToIncome,
            score: -40,
            notes: format!(
                "ratio {:.2} exceeds required {:.2}",
                rent_to_income, config.minimum_rent_to_income_ratio
            ),
        });
        total_score -= 40;
    }

    let credit_score = profile.credit_score;
    if let Some(min_credit) = config.minimum_credit_score {
        match credit_score {
            Some(score) if score >= min_credit => {
                components.push(ScoreComponent {
                    factor: LawfulFactorKind::CreditScore,
                    score: 20,
                    notes: format!("credit score {score} meets minimum {min_credit}"),
                });
                total_score += 20;
            }
            Some(score) => {
                components.push(ScoreComponent {
                    factor: LawfulFactorKind::CreditScore,
                    score: -25,
                    notes: format!("credit score {score} below minimum {min_credit}"),
                });
                total_score -= 25;
            }
            None => {
                components.push(ScoreComponent {
                    factor: LawfulFactorKind::CreditScore,
                    score: -10,
                    notes: "missing credit history".to_string(),
                });
                total_score -= 10;
            }
        }
    }

    let eviction_count = profile
        .lawful_factors
        .get(&LawfulFactorKind::RentalHistory)
        .and_then(|value| match value {
            LawfulFactorValue::Count(count) => Some(*count as u8),
            _ => None,
        })
        .unwrap_or_else(|| {
            profile
                .rental_history
                .iter()
                .filter(|reference| reference.filed_eviction)
                .count() as u8
        });

    if eviction_count == 0 {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::RentalHistory,
            score: 10,
            notes: "no prior evictions".to_string(),
        });
        total_score += 10;
    } else if eviction_count <= config.max_evictions {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::RentalHistory,
            score: -10,
            notes: format!("{eviction_count} eviction(s) within policy"),
        });
        total_score -= 10;
    } else {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::RentalHistory,
            score: -25,
            notes: format!("{eviction_count} eviction(s) exceeds allowance"),
        });
        total_score -= 25;
    }

    if let Some(LawfulFactorValue::Decimal(coverage)) = profile
        .lawful_factors
        .get(&LawfulFactorKind::VoucherCoverage)
    {
        if *coverage > 0.0 {
            components.push(ScoreComponent {
                factor: LawfulFactorKind::VoucherCoverage,
                score: 5,
                notes: format!("voucher covers {:.0}% of rent", coverage * 100.0),
            });
            total_score += 5;
        }
    }

    if let Some(LawfulFactorValue::Boolean(true)) = profile
        .lawful_factors
        .get(&LawfulFactorKind::IowaSecurityDepositCompliance)
    {
        components.push(ScoreComponent {
            factor: LawfulFactorKind::IowaSecurityDepositCompliance,
            score: 5,
            notes: "security deposit within Iowa cap".to_string(),
        });
        total_score += 5;
    }

    let mut violent_felony = None;
    for record in &profile.criminal_history {
        if record.classification == CriminalClassification::ViolentFelony
            && record.years_since <= config.violent_felony_lookback_years
        {
            violent_felony = Some(record.description.clone());
            break;
        }
    }

    let signals = ScoreSignals {
        rent_to_income,
        credit_score,
        eviction_count,
        violent_felony,
    };

    (components, total_score, signals)
}
