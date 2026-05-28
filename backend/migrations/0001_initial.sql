-- Initial schema for factoryplan-rust
-- Decisions reference: docs/PLAN.md §3

PRAGMA foreign_keys = ON;

CREATE TABLE scenario (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE factory (
    id          TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL REFERENCES scenario(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    bays        INTEGER NOT NULL CHECK (bays >= 0)
);
CREATE INDEX idx_factory_scenario ON factory(scenario_id);

CREATE TABLE product (
    id          TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL REFERENCES scenario(id) ON DELETE CASCADE,
    name        TEXT NOT NULL
);
CREATE INDEX idx_product_scenario ON product(scenario_id);

CREATE TABLE product_lead_time (
    id              TEXT PRIMARY KEY,
    product_id      TEXT NOT NULL REFERENCES product(id) ON DELETE CASCADE,
    year            INTEGER NOT NULL,
    quarter         INTEGER NOT NULL CHECK (quarter BETWEEN 1 AND 4),
    lead_time_days  INTEGER NOT NULL CHECK (lead_time_days > 0),
    UNIQUE (product_id, year, quarter)
);
CREATE INDEX idx_lead_time_product ON product_lead_time(product_id);

CREATE TABLE demand (
    id              TEXT PRIMARY KEY,
    scenario_id     TEXT NOT NULL REFERENCES scenario(id) ON DELETE CASCADE,
    product_id      TEXT NOT NULL REFERENCES product(id) ON DELETE CASCADE,
    period_type     TEXT NOT NULL CHECK (period_type IN ('month', 'quarter')),
    year            INTEGER NOT NULL,
    period_index    INTEGER NOT NULL,   -- 1..12 for month, 1..4 for quarter
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    spread_mode     TEXT NOT NULL DEFAULT 'even'
                    CHECK (spread_mode IN ('even', 'start', 'end'))
);
CREATE INDEX idx_demand_scenario ON demand(scenario_id);
CREATE INDEX idx_demand_product  ON demand(product_id);

CREATE TABLE schedule_run (
    id                  TEXT PRIMARY KEY,
    scenario_id         TEXT NOT NULL REFERENCES scenario(id) ON DELETE CASCADE,
    run_at              TEXT NOT NULL,
    total_demand        INTEGER NOT NULL,
    shipped_on_time     INTEGER NOT NULL,
    unshippable         INTEGER NOT NULL
);
CREATE INDEX idx_run_scenario ON schedule_run(scenario_id);

CREATE TABLE scheduled_unit (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES schedule_run(id) ON DELETE CASCADE,
    demand_id       TEXT NOT NULL,
    product_id      TEXT NOT NULL,
    factory_id      TEXT,           -- null when unshippable
    bay_index       INTEGER,        -- null when unshippable
    required_start  TEXT NOT NULL,  -- ISO date
    due_date        TEXT NOT NULL,  -- ISO date
    status          TEXT NOT NULL CHECK (status IN ('shipped', 'unshippable'))
);
CREATE INDEX idx_unit_run ON scheduled_unit(run_id);

CREATE TABLE recommendation (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES schedule_run(id) ON DELETE CASCADE,
    rec_type        TEXT NOT NULL
                    CHECK (rec_type IN ('bays_needed', 'uniform_lt_pct', 'per_product_lt')),
    payload_json    TEXT NOT NULL
);
CREATE INDEX idx_rec_run ON recommendation(run_id);
