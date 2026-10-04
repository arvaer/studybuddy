-- One row per wake of the operator (21d): what context was handed to the
-- model and what it cost, read off the call as it crosses, so the context
-- policy is chosen from records rather than opinion. The record itself
-- holds the full request and reply; this is the measure beside it.
CREATE TABLE wakes (
    id                 UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id       UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    effect_id          TEXT        NOT NULL,
    woke_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    latency_ms         INTEGER     NOT NULL,
    request_bytes      INTEGER     NOT NULL,
    turns              INTEGER     NOT NULL,
    reads              INTEGER     NOT NULL,
    input_tokens       INTEGER,
    cache_read_tokens  INTEGER,
    cache_write_tokens INTEGER,
    output_tokens      INTEGER,
    picked             TEXT        NOT NULL
);

CREATE INDEX wakes_workspace_idx ON wakes (workspace_id, woke_at);
