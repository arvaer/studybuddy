-- Goals side by side (22a): each goal is its own run of the coach on the
-- workspace's one record, and the record says which run a park belongs to
-- (its origin, the form that began the run). This row ties a goal to that
-- form. `task` is the run's argument, kept before the run starts so the
-- run can be named again after a crash (run_once with the same id and the
-- same task answers the run it made and writes nothing); `run_form` is
-- filled once the run is known; `ended`/`summary` once it ends. A goal set
-- before 22a has no task: it is adopted from the record as it stands.
CREATE TABLE goal_runs (
    goal_id      UUID        PRIMARY KEY REFERENCES goals(id) ON DELETE CASCADE,
    workspace_id UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    task         TEXT,
    run_form     TEXT,
    ended        TEXT,
    summary      TEXT,
    UNIQUE (workspace_id, run_form)
);
