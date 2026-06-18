-- Per-(product, factory) lead-time overrides.
-- The base lead time stays in product_lead_time (keyed by product + year/quarter).
-- A row here overrides the lead time for a SPECIFIC factory, product and
-- (year, quarter). When no row matches, the scheduler falls back to the base
-- product lead time, so by default every factory shares the same lead time.

PRAGMA foreign_keys = ON;

CREATE TABLE product_factory_lead_time (
    id              TEXT PRIMARY KEY,
    product_id      TEXT NOT NULL REFERENCES product(id)  ON DELETE CASCADE,
    factory_id      TEXT NOT NULL REFERENCES factory(id)  ON DELETE CASCADE,
    year            INTEGER NOT NULL,
    quarter         INTEGER NOT NULL CHECK (quarter BETWEEN 1 AND 4),
    lead_time_days  INTEGER NOT NULL CHECK (lead_time_days > 0),
    UNIQUE (product_id, factory_id, year, quarter)
);
CREATE INDEX idx_pflt_product ON product_factory_lead_time(product_id);
CREATE INDEX idx_pflt_factory ON product_factory_lead_time(factory_id);
