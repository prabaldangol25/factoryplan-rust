# factoryplan-rust

Finite-capacity scheduling + Gantt visualization tool. Given demand, factories
(with bay counts), and products (with per-quarter lead times), backward-schedules
every unit and reports:

- Which units ship on time
- Which units are **unshippable** given current constraints
- Recommended ways to clear shortfall:
  - Bays to add (and where)
  - Uniform % lead-time reduction
  - Per-product lead-time targets

## Status

🚧 Planning phase complete. See [`docs/PLAN.md`](docs/PLAN.md) for the full design.

Implementation has not started yet.

## Tech stack

- **Backend**: Rust + Actix-web + SQLite (sqlx)
- **Frontend**: React 19 + TypeScript + Vite + Tailwind + Plotly
- **Deploy**: Docker + Fly.io + Netlify

## Roadmap

| Phase | Scope | Status |
|---|---|---|
| 0 | Scaffolding | ⏳ Pending |
| 1 | Data model + CRUD | ⏳ Pending |
| 2 | Scheduling core | ⏳ Pending |
| 3 | Recommendations | ⏳ Pending |
| 4 | Results UI | ⏳ Pending |
| 5 | Excel import + export | ⏳ Pending |
| 6 | Polish + deploy | ⏳ Pending |
