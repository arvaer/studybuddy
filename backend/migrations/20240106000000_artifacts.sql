-- Artifacts (#11): every uploaded file is an immutable, content-addressed
-- record. Bytes live at uploads/artifacts/<sha256[0..2]>/<sha256>; a later
-- upload with the same filename is a new artifact and cannot touch an
-- earlier one. See docs/artifacts.md.

CREATE TABLE artifacts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sha256            TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    size_bytes        BIGINT NOT NULL CHECK (size_bytes >= 0),
    content_type      TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    uploaded_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX artifacts_user_idx ON artifacts (user_id, uploaded_at DESC);
CREATE INDEX artifacts_sha_idx  ON artifacts (sha256);

CREATE TRIGGER artifacts_immutable
    BEFORE UPDATE ON artifacts
    FOR EACH ROW EXECUTE FUNCTION forbid_update();

-- An uploaded resource points at its artifact. Rows from before this
-- migration keep file_path only and artifact_id NULL.
ALTER TABLE resources ADD COLUMN artifact_id UUID REFERENCES artifacts(id);
CREATE INDEX resources_artifact_idx ON resources (artifact_id);

-- A revision pins the exact source version it was authored against. It is
-- copied from the resource at authoring time, so re-pointing the resource
-- later cannot change what the revision cites. NO ACTION on delete: an
-- artifact a revision cites cannot be removed.
ALTER TABLE activity_revisions ADD COLUMN source_artifact_id UUID REFERENCES artifacts(id);
CREATE INDEX activity_revisions_artifact_idx ON activity_revisions (source_artifact_id);
