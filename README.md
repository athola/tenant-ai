# tenant.ai

tenant.ai is a Rust-based web service that unifies automated workflows across text, email, phone, and AppFolio tenant management. The initial binary is a lightweight Axum server that will evolve into a multi-channel automation hub for property management teams.

## Getting Started

```bash
cargo run
```

The binary defaults to the HTTP server and exposes `/health` alongside the vacancy report route. Override the network bindings when you need to front the API through a tunnel or container:

```bash
cargo run -- serve --host 0.0.0.0 --port 4000
```

The CLI subcommands below can be invoked from any shell without starting the HTTP service.

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

Key fields inside the `insights` object power the readiness dashboard, automation triggers, and recommended actions surfaced in demos. A sample payload lives in `docs/VACANCY_INSIGHTS.md`.

## Vacancy Workflow CLI Playbook

The CLI ships with an interactive playbook that powers investor and stakeholder demos without needing to wire the UI. The binary defaults to the HTTP server (`cargo run`), but the `vacancy` and `demo` subcommands expose every workflow variation we currently support.

### Vacancy report variations

```bash
# Baseline blueprint (no external data) with summary insights
cargo run -- vacancy report \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08

# Apollo/AppFolio sourced snapshot with overdue focus and task drill-down
cargo run -- vacancy report \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --apollo-csv crates/tenant-ai/Apollo_Apartments.csv \
  --today 2025-10-02 \
  --list-tasks

# Patched storyline used in readiness pitches (progress vs. overdue contrast)
cargo run -- vacancy report \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --apollo-csv crates/tenant-ai/Apollo_Apartments_patched.csv \
  --today 2025-10-02 \
  --list-tasks
```

- `--vacancy-start` / `--target-move-in` anchor the workflow window used for readiness scoring.
- `--today` lets you stress-test overdue logic or simulate historical reporting snapshots.
- `--apollo-csv` hydrates the instance with AppFolio/Apollo exports, so completions, due dates, and compliance alerts mirror live portfolios.
- The patched CSV (`crates/tenant-ai/Apollo_Apartments_patched.csv`) highlights a partially completed run with clear ready vs. in-progress signals for pitch decks.
- `--list-tasks` prints every task instance with stage, owner, due date, and completion markers for deep-dive conversations.

The generated insights include stage progress, role workload, overdue and compliance sections, AI observations, recommended automations, blockers, and automation triggers (see `docs/VACANCY_INSIGHTS.md`). This mirrors the readiness dashboard feed and matches the super.ai readiness narratives around SLA, automation coverage, and compliance nudges.

### End-to-end demo mode

```bash
# Full automation storyline with Apollo snapshot and detailed tasks
cargo run -- demo \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --apollo-csv crates/tenant-ai/Apollo_Apartments.csv \
  --include-tasks

# Showcase the patched readiness arc with explicit overdue remediation
cargo run -- demo \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --apollo-csv crates/tenant-ai/Apollo_Apartments_patched.csv \
  --today 2025-10-02 \
  --include-tasks

# Run the vacancy narrative only (skip the application intake segment)
cargo run -- demo --skip-application

# Use local files for marketing assets and reports
cargo run -- demo \
  --vacancy-start 2025-09-24 \
  --target-move-in 2025-10-08 \
  --photos-dir /path/to/unit/photos \
  --output-dir /path/to/save/reports
```

- `--include-tasks` mirrors the `--list-tasks` mode for investors who want evidence of every automation hook.
- `--skip-application` isolates the vacancy workflow for quick operational briefings; combine it with `--apollo-csv` for snapshot-only pitches.
- Vacancy dates, `--today`, and the CSV path all accept overrides so you can align with any portfolio timeline.
- The demo adds a communication automation snapshot (per-channel lead counts, automation rates, SLA attainment, escalations) to show omnichannel coverage on par with super.ai’s leasing assistant positioning.
- When the application intake segment runs, the CLI walks through submission, evaluation, score breakdown, rent-to-income calculations, public status payloads, and alert fan-out. This exceeds the scope of many AppFolio-integrated competitors who stop at lead response.
- `--photos-dir` tells the demo to use photos from a local directory. The directory should contain image files (e.g., `kitchen.jpg`, `bedroom.png`) for the property.
- `--output-dir` specifies a directory where the generated marketing report (an HTML file) will be saved.
- This local file workflow replaces the previous Google Drive integration, allowing for easy offline demos with real assets.

### HTTP parity

Everything the CLI prints is available through `/api/v1/vacancy/report`, so teams can toggle between CLI demos and API-driven dashboards without divergence.

## Competitive positioning

| Capability | Agentic Property Orchestrator CLI | super.ai AppFolio orchestration | Other AI property platforms (e.g., EliseAI, Funnel, MeetElise) |
| --- | --- | --- | --- |
| Vacancy readiness scoring | Stage progress, role workload, blockers, readiness levels, automation triggers, and recommended actions in one command | Focused on SLA and communication metrics; vacancy task coverage is opaque | Varies; most surface basic task counts or SLA widgets without automation recommendations |
| AppFolio/AppFolio-export ingestion | First-class CSV importer (`--apollo-csv`) that hydrates task status, compliance, and due dates for live pilots | Requires direct API hookups; offline demos rely on canned screenshots | Typically needs staging sandboxes; few support offline demo data |
| Communication automation metrics | Synthetic but parameterized lead automation report with per-channel SLAs and escalation counts | Core competency, matched by our CLI output to prove parity | Often limited to email/SMS split without vacancy context |
| Application evaluation workflow | CLI submits, evaluates, scores, and emits alert payloads with lawful-factor transparency | Generally deferred to integrations or partner point solutions | Some offer credit pre-screening, but not full lawful-factor scoring in demo tooling |
| Operator configurability | Toggle dates, data sources, task verbosity, and application flow from CLI flags | Configuration usually lives in web UI; CLI/storytelling tooling is minimal | Demo tooling often requires engineering support to reconfigure |

The CLI-centric approach gives investor teams a portable artifact that demonstrates AppFolio-aware automation and evaluation breadth within minutes. Feature-for-feature, the demo narrative meets or exceeds the standard set by super.ai while highlighting capabilities (lawful-factor scoring transparency, compliance alert surfacing, and offline AppFolio data hydration) that other orchestration platforms typically reserve for paid pilots.

## Documentation

Project plans live under `docs/` and cover the roadmap, architecture, and milestone breakdowns for upcoming merge requests.
