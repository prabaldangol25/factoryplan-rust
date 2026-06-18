-- Roll-forward / backlog model.
--
-- A unit that can't ship in its demanded quarter rolls forward to the next
-- quarter (building from that quarter's first day) and ships "late". We track:
--   schedule_run.shipped_late : count of units that shipped in a later quarter
--   scheduled_unit.orig_due_date : the originally demanded due date
--   scheduled_unit.is_late : 1 if it shipped later than its demanded quarter
--   quarter_miss : per-quarter count of units that missed that quarter and rolled
--
-- status stays 'shipped' | 'unshippable'; a late unit is status='shipped' + is_late=1.

ALTER TABLE schedule_run ADD COLUMN shipped_late INTEGER NOT NULL DEFAULT 0;

ALTER TABLE scheduled_unit ADD COLUMN orig_due_date TEXT;
ALTER TABLE scheduled_unit ADD COLUMN is_late INTEGER NOT NULL DEFAULT 0;

CREATE TABLE quarter_miss (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES schedule_run(id) ON DELETE CASCADE,
    year         INTEGER NOT NULL,
    quarter      INTEGER NOT NULL,
    missed_count INTEGER NOT NULL
);
CREATE INDEX idx_quarter_miss_run ON quarter_miss(run_id);
