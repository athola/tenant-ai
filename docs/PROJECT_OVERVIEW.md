# Project Overview

## Vision
Build a unified automation layer that triages inbound tenant communications, synchronizes with AppFolio, and orchestrates downstream maintenance, leasing, and financial workflows without constant human intervention.

## Objectives
- Consolidate email, SMS, and telephony events into a single conversation timeline per unit or tenant.
- Classify inbound incidents using agentic workflows and direct routine tickets to automation while escalating edge cases to staff.
- Keep AppFolio authoritative for work orders, payments, turnover, and leasing milestones via bi-directional syncs.
- Deliver auditability with traceable workflows, retry logic, and operational dashboards.

## Success Metrics
- 70% reduction in manual follow-up per tenant issue.
- <5 minute average response time for critical maintenance triggers.
- Automated handling of routine rent payment reminders and lease renewal nudges.
- SLA compliance dashboards covering maintenance lifecycle, vacancy readiness, and onboarding flow.

## Project Plan
1. Settle on the recommended-action wording called out in `docs/VACANCY_INSIGHTS.md` and update the automation copy once approved.
2. Sequence the persistence work so readiness snapshots start landing in Postgres and can drive the owner pulse digests.
