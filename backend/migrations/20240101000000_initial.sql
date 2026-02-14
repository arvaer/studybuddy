-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ─────────────────────────────────────────
-- Users & Auth
-- ─────────────────────────────────────────

CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email        TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,   -- crypt(password, gen_salt('bf', 12))
    display_name TEXT NOT NULL,
    avatar       TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX users_email_idx ON users (email);

CREATE TABLE refresh_tokens (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,  -- sha256 of the raw token
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX refresh_tokens_user_idx ON refresh_tokens (user_id);
CREATE INDEX refresh_tokens_hash_idx ON refresh_tokens (token_hash);

-- ─────────────────────────────────────────
-- Topics
-- ─────────────────────────────────────────

CREATE TABLE topics (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    color       TEXT NOT NULL DEFAULT 'hsl(210, 100%, 50%)',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX topics_user_idx ON topics (user_id);

-- ─────────────────────────────────────────
-- Concepts
-- ─────────────────────────────────────────

CREATE TABLE concepts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    topic_id    UUID REFERENCES topics(id) ON DELETE SET NULL,
    parent_id   UUID REFERENCES concepts(id) ON DELETE SET NULL,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX concepts_user_idx   ON concepts (user_id);
CREATE INDEX concepts_topic_idx  ON concepts (topic_id);
CREATE INDEX concepts_parent_idx ON concepts (parent_id);

-- ─────────────────────────────────────────
-- Reinforcement Units
-- ─────────────────────────────────────────

CREATE TYPE ru_state AS ENUM (
    'introduced',
    'reinforced',
    'unstable',
    'stabilizing',
    'stable',
    'superseded'
);

CREATE TABLE reinforcement_units (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    concept_id          UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    claim               TEXT NOT NULL,
    context             TEXT NOT NULL DEFAULT '',
    state               ru_state NOT NULL DEFAULT 'introduced',
    stability_score     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    reinforcement_count INTEGER NOT NULL DEFAULT 0,
    last_reinforced     TIMESTAMPTZ,
    -- SRS scheduling fields (SM-2 variant)
    ease_factor         DOUBLE PRECISION NOT NULL DEFAULT 2.5,
    interval_days       INTEGER NOT NULL DEFAULT 0,
    due_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ru_concept_idx ON reinforcement_units (concept_id);
CREATE INDEX ru_due_idx     ON reinforcement_units (due_at);

-- RU dependency graph (many-to-many self-ref)
CREATE TABLE ru_dependencies (
    ru_id        UUID NOT NULL REFERENCES reinforcement_units(id) ON DELETE CASCADE,
    depends_on_id UUID NOT NULL REFERENCES reinforcement_units(id) ON DELETE CASCADE,
    PRIMARY KEY (ru_id, depends_on_id)
);

-- ─────────────────────────────────────────
-- Questions
-- ─────────────────────────────────────────

CREATE TYPE question_type AS ENUM ('recall', 'application', 'disambiguation');

CREATE TABLE questions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ru_id          UUID NOT NULL REFERENCES reinforcement_units(id) ON DELETE CASCADE,
    question_type  question_type NOT NULL,
    prompt         TEXT NOT NULL,
    options        JSONB,           -- string[] for multiple choice, null for free-form
    correct_answer TEXT NOT NULL,
    explanation    TEXT NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX questions_ru_idx   ON questions (ru_id);
CREATE INDEX questions_type_idx ON questions (question_type);

-- ─────────────────────────────────────────
-- Quiz Sessions
-- ─────────────────────────────────────────

CREATE TABLE quiz_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    config          JSONB NOT NULL,
    score           INTEGER,
    total_questions INTEGER,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX quiz_sessions_user_idx ON quiz_sessions (user_id);

CREATE TABLE quiz_answers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id  UUID NOT NULL REFERENCES quiz_sessions(id) ON DELETE CASCADE,
    question_id UUID NOT NULL REFERENCES questions(id),
    answer      TEXT NOT NULL,
    is_correct  BOOLEAN NOT NULL,
    answered_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX quiz_answers_session_idx ON quiz_answers (session_id);

-- ─────────────────────────────────────────
-- Study Sessions
-- ─────────────────────────────────────────

CREATE TYPE study_session_type AS ENUM ('reading', 'video', 'quiz');

CREATE TABLE study_sessions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    session_type study_session_type NOT NULL,
    progress     INTEGER NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    started_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX study_sessions_user_idx ON study_sessions (user_id);

CREATE TABLE study_session_concepts (
    session_id UUID NOT NULL REFERENCES study_sessions(id) ON DELETE CASCADE,
    concept_id UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, concept_id)
);

-- ─────────────────────────────────────────
-- Notes
-- ─────────────────────────────────────────

CREATE TABLE notes (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    concept_id       UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    ru_id            UUID REFERENCES reinforcement_units(id) ON DELETE SET NULL,
    content          TEXT NOT NULL,
    is_ai_generated  BOOLEAN NOT NULL DEFAULT false,
    anchor_position  INTEGER,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX notes_user_idx    ON notes (user_id);
CREATE INDEX notes_concept_idx ON notes (concept_id);
CREATE INDEX notes_ru_idx      ON notes (ru_id);

-- ─────────────────────────────────────────
-- Resources
-- ─────────────────────────────────────────

CREATE TYPE resource_type AS ENUM ('pdf', 'video', 'article', 'lecture', 'textbook');

CREATE TABLE resources (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    topic_id      UUID NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    title         TEXT NOT NULL,
    resource_type resource_type NOT NULL,
    url           TEXT,
    added_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX resources_user_idx  ON resources (user_id);
CREATE INDEX resources_topic_idx ON resources (topic_id);

CREATE TABLE resource_concepts (
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    concept_id  UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    PRIMARY KEY (resource_id, concept_id)
);

-- ─────────────────────────────────────────
-- User Settings
-- ─────────────────────────────────────────

CREATE TABLE user_settings (
    user_id                UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    reinforcement_prompts  BOOLEAN NOT NULL DEFAULT true,
    question_frequency     INTEGER NOT NULL DEFAULT 50,
    ai_generated_notes     BOOLEAN NOT NULL DEFAULT true,
    study_time_goal        INTEGER NOT NULL DEFAULT 60,
    daily_questions        INTEGER NOT NULL DEFAULT 20,
    daily_reminders        BOOLEAN NOT NULL DEFAULT false,
    streak_alerts          BOOLEAN NOT NULL DEFAULT true,
    review_reminders       BOOLEAN NOT NULL DEFAULT true,
    reduce_animations      BOOLEAN NOT NULL DEFAULT false
);
