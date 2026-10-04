-- What the learner read (21c): one row per stay on a page of one of their
-- sources, reported by the reading page. Undelivered rows ride on the next
-- attempt's receipt to the operator and are marked delivered in the same
-- transaction, so the coach sees each stay once, with the attempt.
CREATE TABLE page_views (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    resource_id  UUID        NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    page         INTEGER     NOT NULL CHECK (page >= 1),
    seconds      INTEGER     NOT NULL CHECK (seconds >= 0 AND seconds <= 3600),
    observed_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ
);

CREATE INDEX page_views_undelivered_idx ON page_views (workspace_id, observed_at) WHERE delivered_at IS NULL;
