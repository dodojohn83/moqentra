-- Align annotation_projects with the domain model.
ALTER TABLE annotation_projects
    ALTER COLUMN dataset_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS dataset_version_id UUID,
    ADD COLUMN IF NOT EXISTS task_type TEXT,
    ADD COLUMN IF NOT EXISTS ontology JSONB NOT NULL DEFAULT '{}';
