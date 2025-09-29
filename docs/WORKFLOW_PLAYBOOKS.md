# Workflow Playbooks

See `docs/APOLLO_APARTMENTS_WORKFLOW_PLAN.md` for the Apollo pilot's detailed vacancy, leasing, maintenance, and collections workflows that expand on the universal playbooks below.

## Maintenance Requests
- Intake: SMS/email/web form triggers ticket creation.
- Classification: Detect emergency keywords, unit damage indicators.
- Automated Steps:
  - Send acknowledgement with ETA.
  - If critical, escalate to on-call vendor and create AppFolio work order.
  - Non-critical -> schedule self-help instructions or vendor scheduling bot.
- Data Sync: Update AppFolio status changes; capture cost estimates.

## Rent Payment Journey
- Monitor AppFolio ledger for due dates, late fees, payment confirmations.
- Pre-due reminders: SMS/email drip campaign.
- Past-due automation: escalate to payment plan, auto-call tree, or staff.
- Reconcile payments and notify tenants + accounting channel.

## Turnover Management
- Triggered by move-out notice or lease end.
- Playbook tasks: inspection scheduling, cleaning crew dispatch, inventory tracking.
- Automate vendor outreach, confirm completion, update readiness score.

## Lease Renewal
- Nudge tenants 90/60/30 days prior to lease expiration.
- Feed market comps into renewal offer generator.
- Track negotiation outcomes and push final agreements into AppFolio.

## New Resident Onboarding
- Checklist: welcome email, utility setup, access credentials, rent autopay.
- Integrate digital signature workflows and orientation scheduling.
- Provide slack/email updates to property managers for exceptions.

## Vacancy Analysis
- Pull occupancy, days-on-market, rent comps.
- Flag units needing promotions or pricing adjustments.
- Blend marketing channel feedback loops for leasing strategy.
- Lean on the `VacancyInsights` payload (see `docs/VACANCY_INSIGHTS.md`) for readiness, blockers, and automation triggers that feed the dashboard cards and demo scripts.
