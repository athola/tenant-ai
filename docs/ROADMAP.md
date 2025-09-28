# Roadmap & Merge Request Breakdown

## Foundation Phase
1. **Bootstrap Infrastructure**
   - Add configuration loader (dotenv + typed config).
   - Wire tracing, metrics, and error handling into `main`.
   - Introduce health and readiness endpoints.
2. **CI/CD Setup**
   - GitHub Actions workflow: lint + test matrix.
   - Cargo fmt/clippy enforcement.
   - Draft container image build recipe.
3. **Data Layer Scaffolding**
   - Integrate Postgres via `sqlx` with migrations.
   - Define core entities: Tenant, Unit, Ticket, Interaction.
   - Abstract repositories behind traits for testability.

## Communication Ingestion
4. **Messaging Webhooks**
   - Twilio SMS + voice webhook handlers with signature validation.
   - Normalize inbound payloads to shared event model.
5. **Email Intake**
   - Configurable inbound email parser (SendGrid or Mailgun).
   - Attach message context to existing tickets when matches are found.
6. **Telephony Transcript Ingestion**
   - Poll/receive call transcripts, attach audio metadata, store for analysis.

## AppFolio Synchronization
7. **Ticket Sync Service**
   - REST client w/ retries, pagination, backoff.
   - Bi-directional sync for maintenance requests.
8. **Financial Events**
   - Mirror rent payment status, late fee detection, and reminders.
9. **Leasing Lifecycle**
   - Sync lease renewals, turnover tasks, and onboarding checklists.

## Agentic Playbooks
10. **Incident Classification**
    - Connect LLM reasoning engine with guardrails; fallback heuristics.
    - Critical vs non-critical routing logic.
11. **Automated Response Templates**
    - Template library with localization and personalization.
    - Trigger responses via SMS/email/phone.
12. **Maintenance Automation**
    - Vendor matching, scheduling, and AppFolio work order updates.

## Operational Excellence
13. **Dashboard & Reporting**
    - Expose metrics endpoint and build minimal UI stub.
14. **Audit & Compliance**
    - Store agent decision logs, user override trails, and retention policies.
15. **Resilience & Scaling**
    - Circuit breakers for integrations, queue backpressure, chaos testing plan.

Each item targets a focused MR to keep reviews manageable and enable parallel workstreams.

## Next Steps
- Replace the in-memory vacancy application adapters with SQLx-backed persistence and transport-specific alert publishers, then document configuration overrides for staging and production.
- Extend the API surface with evaluation triggers, Prometheus counters, and integration tests covering the `/api/v1/vacancy/applications` endpoints before opening the pilot.
- Shape the follow-on merge requests for AppFolio sync so vacancy applications, leasing, and payment automations share the workflow primitives and background evaluation jobs.
