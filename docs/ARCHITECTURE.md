# Architecture Blueprint

## Service Boundaries
- **Gateway API (Axum)**: Receives inbound events, exposes REST hooks for messaging providers, AppFolio webhooks, and internal dashboards.
- **Workflow Orchestrator**: Routes events through agentic decision trees; bridges LLM tools, rule engines, and legacy automation scripts.
- **Integration Connectors**:
  - Messaging: Twilio (SMS/voice), SendGrid (email) abstractions.
  - AppFolio: Ticket lifecycle, leasing, payments.
  - Payments/Rent: Stripe or AppFolio ledger as long-term single source.
- **State & Storage**:
  - Postgres for operational data and workflow state.
  - Redis (or equivalent) for queues, rate limiting, and transient agent context.
- **Observability**: Tracing via `tracing` crate, metrics export (Prometheus), structured logging to capture agent decisions.

## Module Layout
```
src/
  main.rs              # Axum bootstrap & DI wiring
  config/              # Environment, secrets, feature flags
  api/                 # HTTP routes, typed request/response models
  workflows/           # Agentic orchestration, playbooks, decision trees
  integrations/
    appfolio/          # REST client, webhooks, polling tasks
    messaging/         # SMS, email, telephony providers
  data/
    models.rs          # Persistent entities & queries
    repositories.rs    # Async traits for storage adapters
  jobs/                # Async background workers & schedulers
  telemetry/           # Logging, metrics, tracing setup
```

## Event Flow
1. External trigger hits `/webhooks/...` or `/events/...` endpoint.
2. Event normalized and persisted with tenant/unit context.
3. Orchestrator selects playbook: maintenance, rent, leasing, or onboarding.
4. Agent pipeline executes actions (LLM reasoning, templated responses, AppFolio mutations).
5. Outcomes pushed back to communications channels and AppFolio tickets.
6. Metrics/logs captured for dashboards.

## Deployment Targets
- Containerized service distributed via Docker + Kubernetes or ECS.
- Background workers co-located; scheduled tasks via CronJob or Cloud task runner.
- Feature flagging to safely roll out new automations per property portfolio.
