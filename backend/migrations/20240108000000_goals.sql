-- ─────────────────────────────────────────
-- Goals (20a): the learner's intent, as revisions. Phase 2 holds one goal
-- per workspace at revision 1; later revisions and several goals in one
-- workspace are gate 3. A goal revision's id is the dedup key of the run
-- that serves it (run_once), so one intent starts one run, once.
-- ─────────────────────────────────────────

CREATE TABLE goals (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    revision     INTEGER     NOT NULL CHECK (revision >= 1),
    intent       TEXT        NOT NULL CHECK (length(btrim(intent)) > 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, revision)
);

CREATE TRIGGER goals_immutable
    BEFORE UPDATE ON goals
    FOR EACH ROW EXECUTE FUNCTION forbid_update();
