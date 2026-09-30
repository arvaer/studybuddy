# Baseline findings

Recorded 2026-09-29 while establishing the [development setup](development.md) for hardening issue #1. These are pre-existing conditions found by running the procedure, not fixes. Each has, or should get, its own issue.

## Blocking a fresh checkout

| Finding | Status |
| --- | --- |
| `backend/.sqlx` offline cache was stale: 15 queries had no cached data, so `SQLX_OFFLINE=true cargo build` failed with 45 errors. | Regenerated and committed in the same PR as this document. |
| README "Running Locally" referenced a `be/` directory, Rust 1.75, and an `OPENAI_API_KEY` the backend never reads. | README now points here. |

## Security (deferred to their workstream issues)

| Finding | Issue |
| --- | --- |
| `backend/.env` is **tracked in git** and contains `JWT_SECRET` and `DATABASE_URL`. `.env` is not in `.gitignore`. | #5 |
| `JWT_SECRET` falls back to a known string when unset. | #5 |
| Request tracing logs full request URIs at DEBUG by default. | #7 |

## Quality (non-blocking)

| Finding | Detail |
| --- | --- |
| Backend has **zero tests**: `cargo test` runs 0 tests across all crates. | #2 |
| Frontend test suite is one placeholder asserting `true`. | #2 |
| `npm run lint` reports 6 errors and 9 warnings in: `src/components/markdown-renderer.tsx`, `src/components/quiz-config-modal.tsx`, `src/components/ui/command.tsx`, `src/components/ui/textarea.tsx`, `src/pages/Auth.tsx`, `tailwind.config.ts`. | Not fixed; lint is not yet a gate. |
| `cargo build` emits one warning (unused `mut`) in the `lugia` binary. | Not fixed. |
| Vite build warns that the main chunk is 1.3 MB minified. | Not fixed. |
| No CI workflow exists; nothing runs the checks automatically. | Consider after #2. |
| Two lockfiles (`package-lock.json`, `bun.lockb`) are committed; this setup uses npm. | Decide and remove one. |

## Verified working

- Migrations apply cleanly from an empty Postgres 16 database (4 migrations).
- Backend compiles online against the isolated database and offline with the regenerated cache.
- Backend starts and serves: unauthenticated `GET /api/topics` returns 401.
- Frontend installs, builds, and its single test passes.
