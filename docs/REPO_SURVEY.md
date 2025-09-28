# Repository Survey

## Vacancy Workflow Touchpoints

```text
CLI / HTTP Entrypoints
        |
        v
Vacancy Workflow Instance <---- ApolloVacancyImporter (optional CSV)
        |
        v
  VacancyReport
        |
        +--> Stage/Role summaries -> HTTP JSON & CLI render
        +--> Compliance alerts -> HTTP JSON & CLI render
```

| Touchpoint | Source | Purpose | Key Artifacts |
| --- | --- | --- | --- |
| Workflow blueprint | `src/workflows/vacancy.rs` (`VacancyWorkflowBlueprint::standard`) | Defines canonical tasks, stages, compliance notes, and due-date rules for vacancy turnover | Task templates (`TaskTemplate`), compliance notes, stage/role enums |
| Workflow instance | `src/workflows/vacancy.rs` (`VacancyWorkflowInstance`) | Materializes task instances for given vacancy window, tracks status updates, produces reports & summaries | `VacancyReport`, `VacancyReportSummary`, compliance alert generation |
| Apollo importer | `src/workflows/apollo.rs` (`ApolloVacancyImporter`) | Maps AppFolio/Apollo CSV exports onto workflow instance to hydrate task status and completion dates | `ApolloRow`, name normalization map, CSV readers |
| HTTP service | `src/main.rs` (`run_server`) | Exposes health, readiness, metrics, and vacancy report API for consumers | Axum router, `vacancy_report_endpoint`, Prometheus metrics |
| CLI commands | `src/main.rs` (`Cli` / `Command`) | Runs HTTP server or generates vacancy report from CLI (with CSV import option) | `vacancy report` subcommand, table printer (`render_vacancy_report`) |
| Telemetry | `src/telemetry.rs` | Configures tracing subscribers & Prometheus exporter | `telemetry::init`, `PrometheusMetricLayer` |
| Configuration | `src/config/mod.rs` | Loads environment-driven configuration & resolves server socket | `AppConfig`, `ServerConfig`, `TelemetryConfig` |
| Error handling | `src/error.rs` | Normalizes service errors into HTTP responses and CLI diagnostics | `AppError` conversions |

## Axum Routes

| Route | Method | Handler | Response |
| --- | --- | --- | --- |
| `/health` | GET | `healthcheck` | `{ "status": "ok" }` JSON |
| `/ready` | GET | `readiness_endpoint` | 200/503 with readiness JSON, tied to startup flag |
| `/metrics` | GET | `metrics_endpoint` | Prometheus text payload |
| `/api/v1/vacancy/report` | POST | `vacancy_report_endpoint` | Vacancy report JSON with stage/role metrics, overdue tasks, compliance alerts, optional task list |

## CLI Commands

| Command | Options | Behavior |
| --- | --- | --- |
| `serve` (default) | `--host`, `--port` | Boot HTTP service with optional overrides, wires telemetry and Prometheus |
| `vacancy report` | `--vacancy-start`, `--target-move-in`, `--today`, `--apollo-csv`, `--list-tasks` | Generates workflow report (standard blueprint or Apollo CSV import), prints stage/role tables, overdue and compliance sections |

## Supporting Docs & Tests

- `docs/WORKFLOW_PLAYBOOKS.md`, `docs/APOLLO_APARTMENTS_WORKFLOW_PLAN.md`: Narrative descriptions of turnover stages and automation goals.
- `tests/apollo_vacancy_import.rs`: Validates CSV mapping & status hydration.
- `tests/vacancy_workflow.rs`: Ensures blueprint composition, reporting behavior, and human-readable summaries.

