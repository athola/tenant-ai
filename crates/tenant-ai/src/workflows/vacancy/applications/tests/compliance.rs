use super::common::*;
use crate::workflows::vacancy::applications::compliance::ComplianceViolation;
use crate::workflows::vacancy::applications::domain::{
    HouseholdComposition, ProhibitedScreeningPractice,
};

#[test]
fn guard_requires_verified_income_sources() {
    let guard = guard();
    let submission = missing_income_submission();

    match guard.profile_from_submission(submission) {
        Err(ComplianceViolation::MissingIncomeDocumentation) => {}
        other => panic!("expected missing income documentation, got {other:?}"),
    }
}

#[test]
fn guard_rejects_zero_household_and_zero_income() {
    let guard = guard();
    let mut submission = submission();
    submission.household = HouseholdComposition {
        adults: 0,
        children: 0,
        bedrooms_required: 0,
    };
    submission.income.gross_monthly_income = 0;

    match guard.profile_from_submission(submission) {
        Err(ComplianceViolation::IncompleteHousehold) => {}
        other => panic!("expected incomplete household violation, got {other:?}"),
    }
}

#[test]
fn guard_enforces_prohibited_screening_practices() {
    let guard = guard();
    let submission = prohibited_submission();

    match guard.profile_from_submission(submission) {
        Err(ComplianceViolation::ProhibitedPractice(
            ProhibitedScreeningPractice::ProtectedClassInquiry { field },
        )) => assert_eq!(field, "disability"),
        other => panic!("expected protected class violation, got {other:?}"),
    }
}
