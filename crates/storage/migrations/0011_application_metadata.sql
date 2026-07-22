-- Align applications/application_versions with the domain model.
ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}';

ALTER TABLE application_versions
    ADD COLUMN IF NOT EXISTS state TEXT NOT NULL DEFAULT 'Draft',
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
