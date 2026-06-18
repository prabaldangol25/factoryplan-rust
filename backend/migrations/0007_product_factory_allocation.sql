-- Product-to-factory allocation rules.
-- year=0, quarter=0 is the global/default rule for a product.
-- A specific (year, quarter) row overrides the global rule for that product.

PRAGMA foreign_keys = ON;

CREATE TABLE product_factory_allocation (
    id              TEXT PRIMARY KEY,
    product_id      TEXT NOT NULL REFERENCES product(id)  ON DELETE CASCADE,
    factory_id      TEXT NOT NULL REFERENCES factory(id)  ON DELETE CASCADE,
    year            INTEGER NOT NULL,
    quarter         INTEGER NOT NULL CHECK (quarter BETWEEN 0 AND 4),
    allocation_pct  INTEGER NOT NULL CHECK (allocation_pct BETWEEN 0 AND 100),
    CHECK ((year = 0 AND quarter = 0) OR (year > 0 AND quarter BETWEEN 1 AND 4)),
    UNIQUE (product_id, year, quarter)
);
CREATE INDEX idx_pfa_product ON product_factory_allocation(product_id);
CREATE INDEX idx_pfa_factory ON product_factory_allocation(factory_id);
