use super::common::*;
use crate::workflows::vacancy::applications::domain::{
    CriminalClassification, CriminalRecord, LawfulFactorKind, LawfulFactorValue,
};
use crate::workflows::vacancy::applications::{ApplicationDecision, DenialReason};

#[test]
fn engine_denies_for_low_credit_history() {
    let engine = evaluation_engine();
    let profile = guard_profile("credit-low", 0.29, Some(550));

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::Denied(DenialReason::AdverseCreditHistory) => {}
        other => panic!("expected adverse credit denial, got {other:?}"),
    }
    assert!(outcome
        .components
        .iter()
        .any(|component| component.factor == LawfulFactorKind::CreditScore));
}

#[test]
fn engine_handles_missing_credit_history() {
    let engine = evaluation_engine();
    let profile = guard_profile("credit-missing", 0.3, None);

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::Denied(DenialReason::AdverseCreditHistory) => {}
        other => panic!("expected adverse credit denial, got {other:?}"),
    }
    assert!(outcome
        .components
        .iter()
        .any(|component| component.factor == LawfulFactorKind::CreditScore && component.score < 0));
}

#[test]
fn engine_awards_points_for_strengths() {
    let engine = evaluation_engine();
    let profile = guard_profile("strong", 0.27, Some(720));

    let outcome = engine.score(&profile);

    assert_eq!(outcome.application_id, profile.application_id);
    assert!(matches!(outcome.decision, ApplicationDecision::Approved));
    assert!(outcome.components.iter().any(|component| {
        component.factor == LawfulFactorKind::RentToIncome && component.score > 0
    }));
    assert!(outcome.total_score > 0);
}

#[test]
fn engine_denies_when_rent_to_income_is_too_high() {
    let engine = evaluation_engine();
    let mut profile = guard_profile("ratio-high", 0.27, Some(712));
    profile.lawful_factors.insert(
        LawfulFactorKind::RentToIncome,
        LawfulFactorValue::Decimal(0.45),
    );

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::Denied(DenialReason::InsufficientIncome {
            required_ratio,
            actual_ratio,
        }) => {
            assert_eq!(
                required_ratio,
                evaluation_config().minimum_rent_to_income_ratio
            );
            assert_eq!(actual_ratio, 0.45);
        }
        other => panic!("expected insufficient income denial, got {other:?}"),
    }
}

#[test]
fn engine_routes_recent_violent_felonies_to_manual_review() {
    let engine = evaluation_engine();
    let mut profile = guard_profile("felony", 0.27, Some(720));
    profile.criminal_history.push(CriminalRecord {
        classification: CriminalClassification::ViolentFelony,
        years_since: 3,
        jurisdiction: "Iowa District Court".to_string(),
        description: "Assault causing serious injury".to_string(),
    });

    let outcome = engine.score(&profile);

    match outcome.decision {
        ApplicationDecision::ManualReview { reasons } => {
            assert!(reasons
                .iter()
                .any(|reason| reason.to_lowercase().contains("violent felony")));
        }
        other => panic!("expected manual review, got {other:?}"),
    }
}
