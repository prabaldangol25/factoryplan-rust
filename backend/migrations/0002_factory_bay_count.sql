-- Per-quarter bay-count overrides for a factory.
-- factory.bays remains the baseline; rows in this table override the baseline
-- for the specified (year, quarter). Missing entries fall back to factory.bays.

CREATE TABLE factory_bay_count (
    id          TEXT PRIMARY KEY,
    factory_id  TEXT NOT NULL REFERENCES factory(id) ON DELETE CASCADE,
    year        INTEGER NOT NULL,
    quarter     INTEGER NOT NULL CHECK (quarter BETWEEN 1 AND 4),
    bays        INTEGER NOT NULL CHECK (bays >= 0),
    UNIQUE (factory_id, year, quarter)
);
CREATE INDEX idx_factory_bay_count_factory ON factory_bay_count(factory_id);
