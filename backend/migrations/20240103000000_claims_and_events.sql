-- Claim-in-Context: first-class entity per spec section 2.2
CREATE TABLE claims_in_context (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  concept_id      UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
  predicate       TEXT NOT NULL,
  supporting_text TEXT NOT NULL DEFAULT '',
  asset_id        UUID REFERENCES resources(id) ON DELETE SET NULL,
  source_location JSONB,
  introduced_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX cic_concept_idx ON claims_in_context (concept_id);
CREATE INDEX cic_asset_idx ON claims_in_context (asset_id);

-- RU now references CiC + gets dependency_cost
ALTER TABLE reinforcement_units
  ADD COLUMN claim_id UUID REFERENCES claims_in_context(id) ON DELETE SET NULL,
  ADD COLUMN dependency_cost DOUBLE PRECISION NOT NULL DEFAULT 0.0;

-- Event log: append-only audit trail (spec section 8)
CREATE TABLE events (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type      TEXT NOT NULL,
  schema_version  INTEGER NOT NULL DEFAULT 1,
  emitted_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  producer        TEXT NOT NULL DEFAULT 'backend',
  user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  aggregate_type  TEXT NOT NULL,
  aggregate_id    UUID NOT NULL,
  sequence        BIGSERIAL,
  payload         JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX events_user_idx ON events (user_id);
CREATE INDEX events_aggregate_idx ON events (aggregate_type, aggregate_id);
CREATE INDEX events_type_idx ON events (event_type);
