-- Add experiment reference to training_jobs for domain mapping.
ALTER TABLE training_jobs
    ADD COLUMN IF NOT EXISTS experiment_id UUID;
