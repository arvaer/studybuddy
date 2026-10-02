-- Capsule record and operator bookkeeping (#18, slice 18a). See
-- docs/phase-2-build.md and docs/capsule-integration.md, "Revisit after
-- Phase 1", decisions 1 and 3.
--
-- The `capsule` schema is what capsule_host::postgres::PgStorage::existing
-- expects, byte for byte: it is created here so that migrations own every
-- table and the application never creates schema at runtime. PgStorage
-- inserts nodes with ON CONFLICT DO NOTHING and moves refs by
-- compare-and-swap; it never updates a node, so nodes get the immutability
-- trigger the learning records use.

CREATE SCHEMA capsule;

CREATE TABLE capsule.nodes (
    address TEXT  PRIMARY KEY,
    node    JSONB NOT NULL
);

CREATE TRIGGER capsule_nodes_immutable
    BEFORE UPDATE ON capsule.nodes
    FOR EACH ROW EXECUTE FUNCTION forbid_update();

CREATE TABLE capsule.refs (
    name    TEXT PRIMARY KEY,
    address TEXT NOT NULL
);

-- ─────────────────────────────────────────
-- Workspaces: the scope an operator session is bound to. One learner may
-- hold several; Phase 2 uses one. Sessions and receipts hang off a row.
-- ─────────────────────────────────────────

CREATE TABLE workspaces (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX workspaces_user_idx ON workspaces (user_id);

-- One capsule session per workspace, and one owner at a time: a process
-- holds the lease while its Owner thread is alive and renews it; a second
-- process finding a live lease refuses to open the session (18b).

CREATE TABLE workspace_sessions (
    workspace_id UUID PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    session_name TEXT NOT NULL UNIQUE,
    owner_lease  UUID,
    lease_until  TIMESTAMPTZ,
    CHECK ((owner_lease IS NULL) = (lease_until IS NULL))
);

-- ─────────────────────────────────────────
-- Effect receipts: one row per effect the application performed for the
-- operator, keyed by the effect id Core hands the provider. A replayed
-- effect id returns this row and performs nothing (18c, 19a). Immutable.
-- ─────────────────────────────────────────

CREATE TABLE effect_receipts (
    effect_id    TEXT PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    family       TEXT NOT NULL,
    payload      JSONB NOT NULL,
    recorded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX effect_receipts_workspace_idx ON effect_receipts (workspace_id, recorded_at);

CREATE TRIGGER effect_receipts_immutable
    BEFORE UPDATE ON effect_receipts
    FOR EACH ROW EXECUTE FUNCTION forbid_update();
