# Agentic Property Orchestrator

Agentic Property Orchestrator is a Rust-based web service that unifies automated workflows across text, email, phone, and AppFolio tenant management. The initial binary is a lightweight Axum server that will evolve into a multi-channel automation hub for property management teams.

## Getting Started

```bash
cargo run
```

The server currently exposes a single `/health` endpoint while deeper workflow integrations are designed in the docs.

## Vacancy API

An HTTP endpoint mirrors the CLI demo and returns the same vacancy readiness insights as JSON:

```bash
curl -X POST http://localhost:3000/api/v1/vacancy/report \
  -H "content-type: application/json" \
  -d '{
        "vacancy_start": "2025-09-24",
        "target_move_in": "2025-10-08",
        "include_tasks": true
      }'
```

Optional fields:

- `today` (string `YYYY-MM-DD`) overrides the evaluation date.
- `apollo_csv` (string) supplies raw CSV content from the Apollo export to hydrate progress and completion dates.
- `include_tasks` toggles the full task listing payload.

The response includes ordered stage progress, role load, compliance alerts, and—when requested—the detailed task breakdown with deliverables and compliance notes.

## Command Line Demo

An interactive CLI is available for quick stakeholder demos and validation. The CLI defaults to running the HTTP server, but you can generate a vacancy readiness report instead:

```bash
cargo run -- vacancy report \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --apollo-csv Apollo_Apartments.csv \
  --list-tasks
```

Key options:

- `--vacancy-start` and `--target-move-in` (required) define the reporting window.
- `--today` optionally overrides the date used to determine overdue work (defaults to the current day).
- `--apollo-csv` hydrates task progress from an Apollo export; omit it to preview the baseline workflow.
- `--list-tasks` adds a chronological task breakdown to the summary report.

## Documentation

Project plans live under `docs/` and cover the roadmap, architecture, and milestone breakdowns for upcoming merge requests.
