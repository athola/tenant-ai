# Apollo Apartments Workflow Plan

## Context
- Source data: `Apollo_Apartments.csv` Asana export dated September 2025 covering vacancy, leasing, maintenance, renewal, and collections tasks.
- Goal: translate portfolio-specific checklists into orchestrator-ready playbooks with clear automation hooks, compliance guardrails, and AppFolio touchpoints.
- Placement: companion to `docs/WORKFLOW_PLAYBOOKS.md`; this document captures the detailed Apollo pilot flow while the main playbook file retains universal guidance.

## Workflow Matrix
- **Vacancy** → marketing, application screening, leasing; hands off to New Resident workflow after keys are transferred.
- **New Resident** → onboarding, LIHTC compliance, resident orientation, and post-move-in follow-up.
- **Maintenance** → intake triage, scheduling/dispatch, and digital close-out of work orders.
- **Turnover** → pre-move-out coordination, inspection, deposit handling, and make-ready loop back into Vacancy workflow.
- **Renewal** → 90/60/30 day cadence for LIHTC recertification, rent updates, and documentation retention.
- **Delinquent Rent** → escalation from reminders through legal action with dependency on Turnover once possession changes.

## Vacancy Workflow
### Stage Map
1. **Marketing & Advertising**: launch listings, update vacancy status in AppFolio, and syndicate to channels.
2. **Screening & Application**: manage inquiries, schedule showings, collect applications, verify documentation, and deliver applicant decisions.
3. **Lease Signing & Move-In Prep**: circulate lease for signature, collect funds, complete inspections, and finalize AppFolio occupancy status.
4. **Handoff**: trigger the New Resident workflow after keys are issued.

### Automation Hooks
- Autogenerate listings and status updates when turnover tasks complete.
- Unified comms queue for inquiry response, scheduling links, and status updates.
- Document checklist automation: request, track, and file income/ID verification artefacts.
- Integrate tenant communication outcomes back into AppFolio and CRM timelines.

### Compliance Notes
- Maintain Fair Housing parity across communications and showings.
- Iowa Code § 562A.29 obligations for re-rental efforts; log marketing actions.
- Ensure leasing documents and funds map to LIHTC constraints before occupancy.

## New Resident Workflow
### Stage Map
1. **Pre-Move-In**: finalize lease artefacts, collect move-in funds, complete LIHTC initial certification, and set up tenant profile in AppFolio.
2. **Welcome & Orientation**: conduct walk-through, share property programs, emergency procedures, and expectations.
3. **Post-Move-In Follow-Up**: confirm unit readiness, promote wellness programming, and archive compliance packets.

### Automation Hooks
- Generate onboarding email drips with tailored program information and QR codes.
- Track outstanding LIHTC documentation and surface alerts for missing items.
- Push move-in inspection artifacts to shared storage and AppFolio folders.

### Compliance Notes
- Security deposit caps (≤ 2 months rent) and VAWA disclosures must be verifiable.
- Maintain signed student status certifications and income calculations for LIHTC audit trails.

## Maintenance Workflow
### Stage Map
1. **Intake & Triage**: receipt acknowledgement, severity scoring, scheduling eligibility.
2. **Scheduling & Dispatch**: notice of entry (24-hour rule), vendor assignment, execution tracking.
3. **Closeout**: collect receipts, photos, and mark work orders complete in AppFolio and Asana.

### Automation Hooks
- Event-driven triage with LLM classification from inbound channels.
- Calendar integration for technician slots and automated notices to tenants.
- Auto-sync work order states and documentation into the orchestrator data store.

### Compliance Notes
- Iowa Code § 562A.19 notice requirements for non-emergency entry.
- Preserve artifact history for auditability and dispute resolution.

## Turnover Workflow
### Stage Map
1. **Pre-Move-Out**: send instructions, share inspection history, schedule walkthrough.
2. **Move-Out Day**: confirm vacancy, capture condition evidence, start deposit ledger.
3. **Security Deposit Processing**: calculate deductions, issue itemized statements, store receipts.
4. **Make-Ready & Handoff**: punch list execution, cleaning, create new move-in inspection, then start Vacancy workflow.

### Automation Hooks
- Automate packet delivery with forwarding address reminders and inspection scheduling links.
- Generate move-out inspection tasks with structured photo logging requirements.
- Start Vacancy workflow once make-ready checklist completes and deposit ledger closes.

### Compliance Notes
- Security deposit disposition within 30 days and proper abandoned property handling.
- Tag damages vs. wear-and-tear to justify deductions.

## Renewal Workflow
### Stage Map
1. **90-Day Notice**: send intent-to-renew and recertification packet.
2. **60-Day Follow-Up**: chase documentation, verify household changes, validate income against AMI limits.
3. **30-45 Day Preparation**: set rent adjustments, issue renewal agreement, manage non-compliance scenarios.
4. **Post-Renewal Closeout**: file TIC updates, sync lease terms, prep owner certification package.

### Automation Hooks
- Time-based reminders for recertification packet return and document verification tasks.
- Automated AMI eligibility calculations with alerting for over-income cases.
- Digital signature workflows for renewal docs and synchronized AppFolio updates.

### Compliance Notes
- Adhere to Iowa Code § 562A.13 rent increase notice requirements.
- Maintain LIHTC recertification documentation and Form 8609 preparation artefacts.

## Delinquent Rent Workflow
### Stage Map
1. **Initial Response**: payment status check, courtesy reminders, late fee calculation.
2. **Notice Phase**: issue 3-day notice with proof of service and method tracking.
3. **Monitoring**: daily follow-ups during notice window, capture tenant interactions.
4. **Escalation**: legal consultation, FED filing, court coordination, writ scheduling, then launch Turnover workflow.

### Automation Hooks
- Payment monitoring tied to ledger events with templated notifications.
- Document assembly for notices and legal packets, including service affidavits.
- Trigger eviction-to-turnover handoff with property manager alerts.

### Compliance Notes
- Ensure statutory notice language/methods are preserved with timestamped evidence.
- Escalation steps must respect lease terms and legal counsel direction.

## Implementation Path
1. **Data Modeling**: extend workflow schemas to represent Apollo stage/task hierarchies, compliance artefacts, and cross-workflow triggers.
2. **Automation Backlog**: prioritize comms automation (Vacancy + Delinquent Rent), AppFolio sync extensions (lease status, inspections), and LIHTC document tracking.
3. **Operational Guardrails**: encode legal notice templates, tenancy timelines, and deposit deadlines into rules/alerting.
4. **Pilot Enablement**: stand up dashboards for Apollo-specific SLAs (marketing turnaround, maintenance close rates, recertification compliance).

## Next Steps
- Extend the importer mapping for Apollo tasks so New Resident, Renewal, and Delinquent Rent stages hydrate alongside the vacancy backbone built in this sprint.
- Define data persistence requirements for workflow snapshots (storage schema, retention policy, export paths) before wiring the sqlx repositories.
- Prototype stakeholder demos that exercise the HTTP vacancy report endpoint with realistic Apollo CSV payloads to validate compliance alert messaging.

## Documentation Follow-Up
- Cross-link this plan from `docs/WORKFLOW_PLAYBOOKS.md` and future property-specific guides.
- Update roadmap epics once implementation tasks are estimated and scheduled.
