-- Content-to-RU Pipeline: schema additions
-- Resources: uploaded file path and cached extracted text
ALTER TABLE resources ADD COLUMN file_path TEXT;
ALTER TABLE resources ADD COLUMN content_text TEXT;

-- RUs: traceability back to the source resource
ALTER TABLE reinforcement_units ADD COLUMN source_resource_id UUID REFERENCES resources(id) ON DELETE SET NULL;
CREATE INDEX ru_source_resource_idx ON reinforcement_units (source_resource_id);
