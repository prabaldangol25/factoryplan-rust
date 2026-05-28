# factoryplan-rust

Finite-capacity production planner. Given demand, factories (with bay counts), and
products (with per-quarter lead times), it **backward-schedules** every unit and
tells you:

- Which units ship on time and which are unshippable given current constraints
- How to clear the shortfall, via three side-by-side recommendations:
  - **Bays to add** (and where)
  - **Uniform % lead-time reduction** across all products
  - **Per-product lead-time targets**

The scheduling model and design decisions are documented in [`docs/PLAN.md`](docs/PLAN.md).

---

## Quick start (local dev)

You need `cargo` (Rust 1.80+) and `npm` (Node 20+). No admin rights or system
libraries needed — SQLite is bundled, Plotly is just an npm package.

```bash
# 1. Start the backend (port 8080, creates ./factoryplan.db on first run)
cd backend
cargo run

# 2. In another terminal, start the frontend dev server (port 5173)
cd frontend
npm install        # first time only
npm run dev

# 3. Open http://localhost:5173 in a browser
```

The frontend dev server proxies `/api/*` to the backend automatically.

---

## What you can do in the UI

1. **Scenarios tab (top bar)** — create, rename, clone, or delete scenarios.
   Each scenario has its own factories / products / demand / runs.
2. **Factories** — name + bay count.
3. **Products** — name + a per-(year, quarter) lead-time matrix.
4. **Demand** — rows specifying *product × period × quantity*. Period can be a
   quarter or a month. Units are exploded evenly (or to start/end) inside the
   period. Bulk-load demand via "Import Excel" (see
   [docs/EXCEL_IMPORT.md](docs/EXCEL_IMPORT.md) for the column layout).
5. **Run** — preview scenario stats and run the scheduler. Results auto-open
   the Results tab.
6. **Results** — recommendation panel, per-period shipment summary, per-factory
   Gantt chart, list of unshippable units, CSV/XLSX download.

---

## API surface

```
GET    /api/health

GET    /api/scenarios
POST   /api/scenarios                       (create / clone via { clone_from })
GET    /api/scenarios/{id}
PUT    /api/scenarios/{id}                  (rename)
DELETE /api/scenarios/{id}
POST   /api/scenarios/{id}/activate

GET    /api/scenarios/{id}/factories
POST   /api/scenarios/{id}/factories
PUT    /api/factories/{id}
DELETE /api/factories/{id}

GET    /api/scenarios/{id}/products
POST   /api/scenarios/{id}/products         (with per-quarter lead_times)
PUT    /api/products/{id}
DELETE /api/products/{id}

GET    /api/scenarios/{id}/demand
POST   /api/scenarios/{id}/demand
POST   /api/scenarios/{id}/demand/import-excel  (multipart file upload)
PUT    /api/demand/{id}
DELETE /api/demand/{id}

POST   /api/scenarios/{id}/run              (executes scheduler + recommendations)
GET    /api/runs/{id}                       (re-fetch persisted run)
GET    /api/runs/{id}/export.csv
GET    /api/runs/{id}/export.xlsx
```

---

## Tech stack

| Layer | Choice |
|---|---|
| Backend | Rust 2021, Actix-web 4, Tokio, sqlx (SQLite) |
| Backend storage | SQLite (file-backed, `factoryplan.db` by default) |
| Excel parsing | calamine |
| XLSX writing | rust_xlsxwriter |
| Frontend | React 19, Vite 5, TypeScript |
| Frontend styling | Tailwind v4 |
| Frontend charts | Plotly.js (react-plotly.js), lazy-loaded |

### Why SQLite

Single file, zero config, copies cleanly, recovers cleanly. The whole point of
v1 is "team planner you can hand someone" — not a multi-tenant SaaS.

### Why Actix-web

Mature, fast, well-supported. The handlers are small enough that swapping
frameworks later would be a few hours of work if needed.

---

## Configuration

The backend reads three environment variables:

| Var | Default | Notes |
|---|---|---|
| `HOST` | `127.0.0.1` | Bind address |
| `PORT` | `8080` | Bind port |
| `DATABASE_URL` | `sqlite://factoryplan.db` | SQLite file path |
| `RUST_LOG` | `info` | Standard `env_logger` filter |

The frontend reads no env vars in dev — Vite proxies `/api/*` to
`http://127.0.0.1:8080`. For a production build, ensure the frontend is served
from the same origin as the backend, or update the proxy / CORS settings.

---

## Tests

```bash
cd backend
cargo test
```

14 unit tests cover:

- Period/quarter math (`quarter_of`, `period_start`, `period_end`)
- Demand explosion (even / start / end spread modes)
- Cross-quarter lead-time selection ("due date's quarter wins")
- Greedy bay placement (empty / fits / capacity-bound / multi-bay)
- All three recommendation algorithms (bays-needed, uniform %, per-product)
- Capacity-bound case (verifies we *don't* recommend a useless LT cut)

---

## Project layout

```
factoryplan-rust/
├── README.md                  ← you are here
├── docs/
│   ├── PLAN.md                ← full design + Q&A trail
│   └── EXCEL_IMPORT.md        ← demand import template
├── backend/
│   ├── Cargo.toml
│   ├── migrations/
│   │   └── 0001_initial.sql
│   └── src/
│       ├── main.rs            ← Actix server + module wiring
│       ├── db.rs              ← sqlx pool + helpers
│       ├── error.rs           ← AppError + ResponseError
│       ├── models.rs          ← DB row + request/response types
│       ├── scheduling.rs      ← backward scheduler + tests
│       ├── recommendations.rs ← three rec algorithms + tests
│       └── handlers/
│           ├── mod.rs
│           ├── scenarios.rs
│           ├── factories.rs
│           ├── products.rs
│           ├── demand.rs
│           ├── runs.rs
│           └── import_export.rs
└── frontend/
    ├── package.json
    ├── vite.config.ts
    └── src/
        ├── main.tsx
        ├── App.tsx            ← tabs + lifted result state
        ├── api/index.ts
        ├── types/index.ts
        └── components/
            ├── ScenarioSwitcher.tsx
            ├── FactoryEditor.tsx
            ├── ProductEditor.tsx
            ├── DemandEditor.tsx
            ├── RunView.tsx
            ├── GanttView.tsx       (lazy-loaded, ~5MB Plotly)
            ├── ShipmentSummary.tsx
            ├── RecommendationPanel.tsx
            ├── UnshippableList.tsx
            └── ErrorBoundary.tsx
```

---

## Deferred to v1.1+ (explicitly out of scope)

- Gap / changeover time between bay reservations
- Per-factory blocked-day windows (maintenance, holidays)
- Authentication / multi-user
- Side-by-side scenario comparison view
- Critical-path / dependency constraints between units
- Persistent run history beyond "latest per scenario"

If any of these become relevant, the scheduler in `scheduling.rs` is the right
extension point — the `BayPool` and `LtIndex` abstractions were designed with
this in mind.

---

## License

Internal / not yet licensed.
