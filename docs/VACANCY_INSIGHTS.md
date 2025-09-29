# Vacancy Insights Reference

## Sample insights payload
The `/api/v1/vacancy/report` endpoint now returns the `insights` object shown below (captured from the standard blueprint evaluated on 2025-09-28 for a 2025-10-08 move-in).

```json
{
  "readiness_score": 0,
  "readiness_level": "at_risk",
  "expected_completion_pct": 0.29,
  "days_until_move_in": 10,
  "days_since_vacancy": 4,
  "focus_stage": "Lease Signing & Move-In",
  "focus_stage_completion": 0.0,
  "blockers": [
    "Create and Publish Listing (Leasing Agent), overdue since 2025-09-24",
    "Update Vacancy Status in AppFolio (Leasing Agent), overdue since 2025-09-24",
    "Manage Inquiries and Schedule Showings (Leasing Agent), overdue since 2025-09-24"
  ],
  "ai_observations": [
    "0 of 10 tasks complete (0% readiness)",
    "5 critical task(s) overdue impacting compliance",
    "Progress is 29% below expected pace for this vacancy window"
  ],
  "recommended_actions": [
    "Concentrate automation on Lease Signing & Move-In (4 open items)",
    "Bundle lease packet tasks and push DocuSign reminders automatically",
    "Escalate compliance checklist to coordinator with documented follow-up"
  ],
  "automation_triggers": [
    "Auto-remind Marketing & Advertising owners of 2 remaining tasks",
    "Auto-remind Screening & Application owners of 3 remaining tasks",
    "Auto-remind Lease Signing & Move-In owners of 4 remaining tasks",
    "Auto-remind Handoff owners of 1 remaining task",
    "Dispatch compliance alerts to AppFolio task queues for overdue work"
  ]
}
```

## Dashboard hooks
- `readiness_score`, `readiness_level`, and `expected_completion_pct` map cleanly to the vacancy readiness gauge on the owner dashboard.
- `blockers` and `ai_observations` seed the “what’s holding us back” card; we surface the first three entries verbatim and keep the rest behind a drill-down.
- `recommended_actions` drive the automation call-to-action banner; tie the first item to the primary CTA button.
- `automation_triggers` power the ops console queue so coordinators can preview which nudges will fire next.

## Recommended action copy feedback
- Does “Concentrate automation on Lease Signing & Move-In (4 open items)” feel natural for leasing leads, or should we invert it to emphasize urgency (e.g., “Escalate move-in prep automation immediately”)?
- Confirm that “Bundle lease packet tasks and push DocuSign reminders automatically” reflects the intended automation outcomes—should we call out SMS nudges as well?
- “Escalate compliance checklist to coordinator with documented follow-up” mentions “coordinator”; verify the correct role title for Apollo before we ship.

## Persistence and owner pulse digests
- Persist each `VacancyInsights` snapshot alongside `vacancy_id`, `captured_on`, and `data_source` in Postgres so we can trend readiness deltas over time.
- Add a nightly job that calculates 7-day readiness trendlines and surfaces slope/volatility metrics for the owner digest.
- Build an “owner pulse” email template that pulls the latest readiness level, top blockers, and any recommended action that has been unchanged for 3 consecutive snapshots.
- When we add persistence, emit a `vacancy_insight.created` event so downstream automations (e.g., Slack alerts, dashboard refresh) can react without polling.
