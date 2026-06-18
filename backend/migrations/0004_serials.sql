-- Per-unit serial numbers.
--
-- Serials are configured on a demand row and assigned to the individual units
-- it explodes into (in due-date order). Two modes:
--   'sequence' : serial_start holds a leading serial whose trailing number is
--                auto-incremented per unit, preserving prefix + zero-padding
--                (e.g. "WID-0010" -> "WID-0011" -> ...).
--   'list'     : serial_list holds an explicit newline-separated list of serials
--                (one per unit), e.g. pasted from an Excel column.
--   'none'     : no serials (default).
--
-- The resolved serial for each scheduled unit is stored on scheduled_unit.

ALTER TABLE demand ADD COLUMN serial_mode TEXT NOT NULL DEFAULT 'none';
ALTER TABLE demand ADD COLUMN serial_start TEXT;
ALTER TABLE demand ADD COLUMN serial_list TEXT;

ALTER TABLE scheduled_unit ADD COLUMN serial TEXT;
