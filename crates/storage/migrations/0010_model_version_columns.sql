-- Align model_versions with the domain model.
ALTER TABLE model_versions
    ADD COLUMN IF NOT EXISTS state TEXT NOT NULL DEFAULT 'Draft',
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
