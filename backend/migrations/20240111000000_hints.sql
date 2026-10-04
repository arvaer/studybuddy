-- Hints (20c). A request is the learner asking, with their draft so far;
-- an unserved request wakes the operator through its wait and is marked
-- served with that wait's receipt. A hint is what the operator answered,
-- written by the learning/hint effect; the attempt that follows carries
-- it as assistance.
CREATE TABLE hint_requests (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id         UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    activity_revision_id UUID        NOT NULL REFERENCES activity_revisions(id) ON DELETE CASCADE,
    draft                TEXT        NOT NULL DEFAULT '',
    requested_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    served_at            TIMESTAMPTZ
);

CREATE INDEX hint_requests_unserved_idx ON hint_requests (activity_revision_id) WHERE served_at IS NULL;

CREATE TABLE hints (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    activity_revision_id UUID        NOT NULL REFERENCES activity_revisions(id) ON DELETE CASCADE,
    text                 TEXT        NOT NULL CHECK (length(btrim(text)) > 0),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX hints_revision_idx ON hints (activity_revision_id, created_at);
