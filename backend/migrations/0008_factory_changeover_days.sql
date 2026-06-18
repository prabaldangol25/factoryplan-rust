ALTER TABLE factory ADD COLUMN changeover_days INTEGER NOT NULL DEFAULT 0 CHECK (changeover_days >= 0);
