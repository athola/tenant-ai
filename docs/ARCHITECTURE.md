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
services/api/
  src/
    main.rs            # thin Tokio bootstrap calling into lib facade
    lib.rs             # public `run` entrypoint used by CLI and tests
    cli.rs             # Clap command tree (`serve`, `demo`, `vacancy report`)
    server.rs          # Axum listener wiring + DI for application services
    routes.rs          # HTTP handlers (health, readiness, metrics, vacancy report)
    demo.rs            # CLI-friendly orchestration + sample data rendering
    infra.rs           # In-memory repositories, alert publisher, date parsing helpers

crates/tenant-ai/
  src/workflows/
    apollo/
      mod.rs           # `ApolloVacancyImporter` facade + error types
      parser.rs        # CSV parsing + test helpers (private, exposed only via cfg(test))
      normalizer.rs    # Name normalization utilities (private)
      mapping.rs       # Apollo -> vacancy task mapping table (private)
    vacancy/
      mod.rs           # Workspace facade for blueprint + report + applications
      blueprint.rs     # Static vacancy workflow definition (task templates)
      domain.rs        # Vacancy domain enums/structs (public module)
      instance.rs      # Runtime workflow instance + task detail projections
      report/
        mod.rs         # Report assembly + insight orchestration (insights kept private)
        summary.rs     # Aggregation logic for stage/role rollups (private)
        views.rs       # DTOs consumed by API clients (`VacancyReportSummary`, etc.)
      applications/
        mod.rs         # Facade re-exporting service, DTOs, router, repository traits
        domain.rs      # Application DTOs shared with HTTP layer
        compliance.rs  # Guard + policy wiring (pub(crate))
        evaluation/
          mod.rs       # Engine + configs (pub(crate) except DTOs)
          config.rs    # Threshold configuration structs
          rules.rs     # Scoring helpers (private)
          policy.rs    # Decision policy evaluation (private)
        repository.rs  # Trait definitions for persistence + alert publishers
        router.rs      # Axum router for `/api/v1/vacancy/applications`
        service.rs     # `VacancyApplicationService` orchestration
        tests/         # Feature-focused unit tests (compliance, evaluation, routing)
```

## Module Boundaries
- `services/api` only depends on the public facades exposed by `tenant_ai::workflows` and keeps
  implementation scaffolding (`infra`, `demo`, `cli`) scoped to `pub(crate)`.
- `tenant_ai::workflows::vacancy` exposes:
  - `VacancyWorkflowBlueprint`, `VacancyWorkflowInstance`, and `VacancyReport` for higher-level
    orchestration.
  - Public `domain` module for read-only DTOs (`VacancyStage`, `TaskStatus`, etc.).
  - `report::views` for HTTP payloads while keeping insight generation private.
- `tenant_ai::workflows::vacancy::applications` re-exports only the service facade, DTOs, router,
  and repository traits. Compliance/evaluation engines remain `pub(crate)` for internal testing
  while still configurable via `EvaluationConfig`.

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
