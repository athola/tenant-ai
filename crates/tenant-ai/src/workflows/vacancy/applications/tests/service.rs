use super::common::*;
use crate::workflows::vacancy::applications::compliance::ComplianceViolation;
use crate::workflows::vacancy::applications::domain::{ApplicationId, VacancyApplicationStatus};
use crate::workflows::vacancy::applications::repository::{
    ApplicationRecord, ApplicationRepository, RepositoryError,
};
use crate::workflows::vacancy::applications::{
    ApplicationDecision, ApplicationServiceError, DenialReason, EvaluationOutcome,
    VacancyApplicationService,
};
use std::sync::Arc;

#[test]
fn submit_propagates_compliance_errors() {
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service =
        VacancyApplicationService::new(repository.clone(), alerts.clone(), evaluation_config());

    let submission = missing_income_submission();

    match service.submit(submission) {
        Err(ApplicationServiceError::Compliance(
            ComplianceViolation::MissingIncomeDocumentation,
        )) => {}
        other => panic!("expected compliance violation, got {other:?}"),
    }
}

#[test]
fn evaluate_sets_under_review_on_manual_review_outcomes() {
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service =
        VacancyApplicationService::new(repository.clone(), alerts.clone(), evaluation_config());

    let record = service
        .submit(manual_review_profile())
        .expect("can submit manual review candidate");
    let outcome = service
        .evaluate(&record.profile.application_id)
        .expect("manual review outcome");

    assert!(matches!(
        outcome.decision,
        ApplicationDecision::ManualReview { .. }
    ));
    let stored = repository
        .fetch(&record.profile.application_id)
        .expect("fetch succeeds")
        .expect("record present");
    assert_eq!(stored.status, VacancyApplicationStatus::UnderReview);
    assert!(
        alerts.events().is_empty(),
        "manual review should not emit alerts"
    );
}

#[test]
fn get_propagates_not_found() {
    let repository = Arc::new(MemoryRepository::default());
    let alerts = Arc::new(MemoryAlerts::default());
    let service = VacancyApplicationService::new(repository, alerts, evaluation_config());

    match service.get(&ApplicationId("missing".to_string())) {
        Err(ApplicationServiceError::Repository(RepositoryError::NotFound)) => {}
        other => panic!("expected not found error, got {other:?}"),
    }
}

#[test]
fn decision_rationale_formats_outcomes() {
    let id = ApplicationId("app-123".to_string());
    let profile = guard_profile("rationale", 0.25, Some(700));

    let approved = ApplicationRecord {
        profile: profile.clone(),
        status: VacancyApplicationStatus::Approved,
        evaluation: Some(EvaluationOutcome {
            application_id: id.clone(),
            decision: ApplicationDecision::Approved,
            total_score: 42,
            components: Vec::new(),
        }),
    };
    assert!(approved.decision_rationale().contains("approved"));

    let conditional = ApplicationRecord {
        profile: profile.clone(),
        status: VacancyApplicationStatus::UnderReview,
        evaluation: Some(EvaluationOutcome {
            application_id: id.clone(),
            decision: ApplicationDecision::ConditionalApproval {
                required_actions: vec!["provide insurance".to_string()],
            },
            total_score: 10,
            components: Vec::new(),
        }),
    };
    assert!(conditional.decision_rationale().contains("conditional"));

    let denied = ApplicationRecord {
        profile: profile.clone(),
        status: VacancyApplicationStatus::Denied,
        evaluation: Some(EvaluationOutcome {
            application_id: id.clone(),
            decision: ApplicationDecision::Denied(DenialReason::InsufficientIncome {
                required_ratio: 0.3,
                actual_ratio: 0.45,
            }),
            total_score: -10,
            components: Vec::new(),
        }),
    };
    assert!(denied.decision_rationale().contains("insufficient income"));

    let manual = ApplicationRecord {
        profile: profile.clone(),
        status: VacancyApplicationStatus::UnderReview,
        evaluation: Some(EvaluationOutcome {
            application_id: id.clone(),
            decision: ApplicationDecision::ManualReview {
                reasons: vec!["income discrepancy".to_string()],
            },
            total_score: 0,
            components: Vec::new(),
        }),
    };
    assert!(manual.decision_rationale().contains("manual review"));

    let pending = ApplicationRecord {
        profile,
        status: VacancyApplicationStatus::Submitted,
        evaluation: None,
    };
    assert_eq!(pending.decision_rationale(), "pending evaluation");
}

#[test]
fn application_status_view_includes_total_score() {
    let id = ApplicationId("app-789".to_string());
    let profile = guard_profile("status-view", 0.22, Some(720));
    let record = ApplicationRecord {
        profile,
        status: VacancyApplicationStatus::Approved,
        evaluation: Some(EvaluationOutcome {
            application_id: id.clone(),
            decision: ApplicationDecision::Approved,
            total_score: 55,
            components: Vec::new(),
        }),
    };

    let view = record.status_view();
    assert_eq!(view.status, VacancyApplicationStatus::Approved.label());
    assert_eq!(view.total_score, Some(55));
    assert!(view.decision_rationale.contains("approved"));
}
