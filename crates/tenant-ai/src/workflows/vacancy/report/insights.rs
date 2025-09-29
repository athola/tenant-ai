use super::super::domain::{TaskStatus, VacancyStage};
use super::super::instance::VacancyWorkflowInstance;
use super::views::{ReadinessLevel, VacancyInsights, VacancyReportSummary};
use chrono::NaiveDate;

pub(crate) fn generate_insights(
    summary: &VacancyReportSummary,
    instance: &VacancyWorkflowInstance,
    vacancy_start: NaiveDate,
    target_move_in: NaiveDate,
    today: NaiveDate,
) -> VacancyInsights {
    let tasks = instance.tasks();
    let total_tasks = tasks.len() as f32;
    let completed_tasks = tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Completed)
        .count() as f32;
    let readiness_score = if total_tasks > 0.0 {
        ((completed_tasks / total_tasks) * 100.0).round()
    } else {
        0.0
    };

    let readiness_score = readiness_score.clamp(0.0, 100.0).round() as u8;

    let overdue_count = summary.overdue_tasks.len();
    let open_tasks = (total_tasks - completed_tasks).max(0.0);
    let days_until_move_in = (target_move_in - today)
        .num_days()
        .clamp(i64::MIN, i64::MAX);
    let days_since_vacancy = (today - vacancy_start).num_days().clamp(i64::MIN, i64::MAX);
    let vacancy_window = (target_move_in - vacancy_start).num_days();
    let expected_completion_pct = if vacancy_window > 0 {
        (days_since_vacancy as f32 / vacancy_window as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let expected_threshold = (expected_completion_pct * 100.0) - 10.0;
    let at_risk_due_to_timing = days_until_move_in <= 0 && open_tasks > 0.0;
    let at_risk_due_to_progress = (readiness_score as f32) < expected_threshold.max(0.0);

    let readiness_level = if readiness_score >= 80 && overdue_count == 0 {
        ReadinessLevel::OnTrack
    } else if readiness_score >= 60 && overdue_count <= 1 && days_until_move_in > 3 {
        ReadinessLevel::Monitor
    } else if at_risk_due_to_timing || at_risk_due_to_progress {
        ReadinessLevel::AtRisk
    } else {
        ReadinessLevel::Monitor
    };

    let focus_stage = summary
        .stage_progress
        .iter()
        .filter(|entry| entry.total > entry.completed)
        .max_by_key(|entry| entry.total - entry.completed);

    let focus_stage_label = focus_stage.map(|entry| entry.stage_label);
    let focus_stage_completion = focus_stage.and_then(|entry| {
        if entry.total == 0 {
            None
        } else {
            Some(entry.completed as f32 / entry.total as f32)
        }
    });

    let mut blockers: Vec<String> = summary
        .overdue_tasks
        .iter()
        .take(3)
        .map(|task| {
            format!(
                "{} ({}), overdue since {}",
                task.name, task.role_label, task.due_date
            )
        })
        .collect();

    if blockers.is_empty() && open_tasks > 0.0 && days_until_move_in <= 3 {
        blockers.push("Move-in is days away with open tasks remaining".to_string());
    }

    let mut ai_observations = Vec::new();
    if total_tasks > 0.0 {
        ai_observations.push(format!(
            "{} of {} tasks complete ({readiness_score}% readiness)",
            completed_tasks as u32, total_tasks as u32
        ));
    }

    if overdue_count > 0 {
        ai_observations.push(format!(
            "{} critical task(s) overdue impacting compliance",
            overdue_count
        ));
    }

    if readiness_score as f32 + 5.0 < expected_completion_pct * 100.0 {
        ai_observations.push(format!(
            "Progress is {:.0}% below expected pace for this vacancy window",
            (expected_completion_pct * 100.0 - readiness_score as f32).round()
        ));
    }

    if days_until_move_in <= 7 {
        ai_observations.push(format!(
            "{} day(s) until target move-in; prioritize move-in readiness",
            days_until_move_in.max(0)
        ));
    }

    let mut recommended_actions = Vec::new();
    if let Some(entry) = focus_stage {
        let outstanding = entry.total.saturating_sub(entry.completed);
        if outstanding > 0 {
            recommended_actions.push(format!(
                "Concentrate automation on {} ({} open item{})",
                entry.stage_label,
                outstanding,
                if outstanding == 1 { "" } else { "s" }
            ));
        }

        match entry.stage {
            VacancyStage::MarketingAndAdvertising => {
                recommended_actions.push(
                    "Refresh listing creative and auto-respond to new leads via SMS & email"
                        .to_string(),
                );
            }
            VacancyStage::ScreeningAndApplication => {
                recommended_actions.push(
                    "Trigger AI-driven applicant nudges and status updates across channels"
                        .to_string(),
                );
            }
            VacancyStage::LeaseSigningAndMoveIn => {
                recommended_actions.push(
                    "Bundle lease packet tasks and push DocuSign reminders automatically"
                        .to_string(),
                );
            }
            VacancyStage::Handoff => {
                recommended_actions
                    .push("Send welcome workflow kickoff with onboarding checklist".to_string());
            }
        }
    }

    if !summary.compliance_alerts.is_empty() {
        recommended_actions.push(
            "Escalate compliance checklist to coordinator with documented follow-up".to_string(),
        );
    }

    if days_until_move_in <= 5 && open_tasks > 0.0 {
        recommended_actions.push(
            "Schedule daily readiness standups until move-in blockers are cleared".to_string(),
        );
    }

    let mut automation_triggers = Vec::new();
    for entry in &summary.stage_progress {
        let outstanding = entry.total.saturating_sub(entry.completed);
        if outstanding > 0 {
            automation_triggers.push(format!(
                "Auto-remind {} owners of {} remaining task{}",
                entry.stage_label,
                outstanding,
                if outstanding == 1 { "" } else { "s" }
            ));
        }
    }

    if overdue_count > 0 {
        automation_triggers.push(
            "Dispatch compliance alerts to AppFolio task queues for overdue work".to_string(),
        );
    }

    if ai_observations.is_empty() {
        ai_observations
            .push("No blockers detected; maintain current automation cadence".to_string());
    }

    VacancyInsights {
        readiness_score,
        readiness_level,
        expected_completion_pct,
        days_until_move_in: days_until_move_in.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        days_since_vacancy: days_since_vacancy.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        focus_stage: focus_stage_label,
        focus_stage_completion,
        blockers,
        ai_observations,
        recommended_actions,
        automation_triggers,
    }
}
