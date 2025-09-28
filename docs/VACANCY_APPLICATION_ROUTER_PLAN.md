# Vacancy Application Router Plan

## Goal
Wire the `application_router` into the public API surface so clients can submit applications, poll status, and trigger evaluations without bypassing the service layer policies.

## Prerequisites
- Concrete repository implementation (initially in-memory, later Postgres).
- Alert publisher adapter stubbed against downstream transport (AppFolio, email, etc.).
- Shared evaluation config loaded from application settings.

## Implementation Steps
1. **State Wiring**
   - Instantiate `ComplianceGuard`, repository adapter, alert publisher, and `EvaluationConfig` inside the HTTP bootstrap (`services/api/src/main.rs`).
   - Construct `VacancyApplicationService` and wrap it in `Arc`.
2. **Router Composition**
   - Mount `application_router(service.clone())` under `/api/v1/vacancy` alongside the existing report endpoint.
   - Ensure layers (tracing, Prometheus metrics, error handlers) apply uniformly across sub-routers.
3. **Evaluation Trigger Endpoint (Future Work)**
   - Extend router with `POST /api/v1/vacancy/applications/:id/evaluate` once evaluation should be callable via API.
   - Consider background job scheduling for automatic evaluations of pending records.
4. **Repository Persistence**
   - Replace memory-backed store with SQLx repository when database layer is ready; keep trait-compatible shim for tests.
5. **Telemetry & Observability**
   - Emit structured events for submissions, evaluations, and alert dispatch attempts.
   - Add metrics counters for `applications_submitted_total`, `applications_approved_total`, etc.
6. **Error Surface & Validation**
   - Return RFC 7807 problem+json payloads for validation errors once shared error stack lands.
   - Harden request schema validation (use `axum::extract::Json` with validation library if needed).
7. **Integration Tests**
   - Expand HTTP tests to cover submission happy path, compliance failures, duplicate detection, and status polling.
   - Add contract tests for evaluation endpoint when available.

## Open Questions
- Should evaluation occur synchronously on submission or via queue/cron? (Impacts route layout.)
- How will authentication/authorization govern application submission endpoints? (Current plan assumes internal use.)
- What downstream alert transports are required for MVP (email, AppFolio task creation, SMS)?

