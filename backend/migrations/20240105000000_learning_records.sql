-- Learning records (#8): owned activities with immutable revisions, immutable
-- attempts against a revision, and explicit assessments. See
-- docs/learning-records.md. Existing tables are untouched; the quiz and
-- question paths keep working until #9/#10 replace them.

-- ─────────────────────────────────────────
-- Immutability: rows in these tables are never updated. Deletes are still
-- allowed so ON DELETE CASCADE from users keeps working.
-- ─────────────────────────────────────────

CREATE FUNCTION forbid_update() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% rows are immutable', TG_TABLE_NAME
        USING ERRCODE = 'integrity_constraint_violation';
END;
$$ LANGUAGE plpgsql;

-- ─────────────────────────────────────────
-- Activities: stable, owned identity. Content lives in revisions.
-- ─────────────────────────────────────────

CREATE TYPE activity_kind AS ENUM ('recall', 'explain', 'apply', 'diagnose');

CREATE TABLE activities (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    concept_id UUID REFERENCES concepts(id) ON DELETE SET NULL,
    kind       activity_kind NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX activities_user_idx    ON activities (user_id);
CREATE INDEX activities_concept_idx ON activities (concept_id);

-- ─────────────────────────────────────────
-- Activity revisions: what the learner was actually shown. Editing an
-- activity inserts a new revision; earlier attempts keep pointing at theirs.
-- ─────────────────────────────────────────

CREATE TABLE activity_revisions (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    activity_id        UUID NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    revision           INTEGER NOT NULL CHECK (revision >= 1),
    prompt             TEXT NOT NULL,
    options            JSONB,          -- string[] for choice items, else NULL
    answer_key         JSONB,          -- deterministic assessment input; NULL means assess by rubric/model
    rubric             TEXT,           -- criteria for a non-deterministic assessment
    source_resource_id UUID REFERENCES resources(id) ON DELETE SET NULL,
    source_location    JSONB,          -- page/offsets today; an artifact address once #12 lands
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (activity_id, revision)
);

CREATE INDEX activity_revisions_source_idx ON activity_revisions (source_resource_id);

CREATE TRIGGER activity_revisions_immutable
    BEFORE UPDATE ON activity_revisions
    FOR EACH ROW EXECUTE FUNCTION forbid_update();

-- ─────────────────────────────────────────
-- Attempts: one immutable record per submission, against the exact revision
-- shown. Carries no correctness column: assessment is a separate record.
-- ─────────────────────────────────────────

CREATE TABLE attempts (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_revision_id UUID NOT NULL REFERENCES activity_revisions(id) ON DELETE CASCADE,
    response             JSONB NOT NULL,                 -- the learner's answer as submitted
    assistance           JSONB NOT NULL DEFAULT '[]',    -- hints, lookups, explanations used
    request_key          TEXT,                            -- client submission key (#10)
    submitted_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX attempts_user_idx     ON attempts (user_id, submitted_at DESC);
CREATE INDEX attempts_revision_idx ON attempts (activity_revision_id);
CREATE UNIQUE INDEX attempts_request_key_idx ON attempts (user_id, request_key)
    WHERE request_key IS NOT NULL;

CREATE TRIGGER attempts_immutable
    BEFORE UPDATE ON attempts
    FOR EACH ROW EXECUTE FUNCTION forbid_update();

-- ─────────────────────────────────────────
-- Assessments: an explicit judgement of an attempt. An attempt with no
-- assessment row is pending, which is not incorrect. A correction inserts
-- a new revision; the original judgement stays.
-- ─────────────────────────────────────────

CREATE TYPE assessment_outcome AS ENUM ('correct', 'partial', 'incorrect');
CREATE TYPE assessment_method  AS ENUM ('exact_match', 'choice', 'model', 'manual');

CREATE TABLE assessments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    attempt_id  UUID NOT NULL REFERENCES attempts(id) ON DELETE CASCADE,
    revision    INTEGER NOT NULL CHECK (revision >= 1),
    outcome     assessment_outcome NOT NULL,
    method      assessment_method NOT NULL,
    score       DOUBLE PRECISION CHECK (score IS NULL OR (score >= 0 AND score <= 1)),
    feedback    TEXT NOT NULL DEFAULT '',
    assessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (attempt_id, revision)
);

CREATE TRIGGER assessments_immutable
    BEFORE UPDATE ON assessments
    FOR EACH ROW EXECUTE FUNCTION forbid_update();

-- The explicit status of every attempt: 'pending' until an assessment
-- exists, otherwise the outcome of the latest assessment revision.
CREATE VIEW attempt_status AS
SELECT a.id                                   AS attempt_id,
       a.user_id,
       COALESCE(s.outcome::TEXT, 'pending')   AS status,
       s.id                                   AS assessment_id
FROM attempts a
LEFT JOIN LATERAL (
    SELECT id, outcome FROM assessments
    WHERE attempt_id = a.id
    ORDER BY revision DESC
    LIMIT 1
) s ON true;
