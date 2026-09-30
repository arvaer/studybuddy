# Ownership: identity through commands into SQL

Audit for the hardening plan's identity-and-ownership workstream (issue #4), done 2026-09-29 by reading every authenticated route, its service call, and the repository SQL it reaches.

## The convention

1. **Authentication is not authorization.** `AuthUser` proves who is calling. It says nothing about whether the caller may touch the record named by a path or body ID.
2. **The learner's `user_id` travels with every command.** A service method that reads or writes an owned record takes `user_id` as a parameter. A handler never binds `AuthUser(_user_id)` and drops it.
3. **Ownership is a SQL predicate, not a separate check.** Every repository method that names a record by ID includes the owner in its `WHERE` clause, either directly (`user_id = $n`) or through the ownership relation (`JOIN concepts c ... AND c.user_id = $n`). Writes that reference another record by ID either use `INSERT ... SELECT ... WHERE <owned>` so the row cannot exist unless the link is owned, or call the helpers in `infra::repositories::owned` first (lists of ids, optional links). Ownership never changes, so check-then-insert has no race.
4. **Foreign equals nonexistent.** A record the caller does not own returns `DomainError::NotFound` (HTTP 404). No response reveals that a record exists for someone else.
5. **No unscoped method survives.** When a lookup is scoped, the unscoped version is removed from the trait, not kept beside it. If nothing can call it, nothing can bypass it.
6. **Tests prove it from the service boundary.** Each domain has a `#[sqlx::test]` file in `backend/crates/infra/tests/` in which learner A owns a record and learner B is refused, including through linked IDs (a question reached via its RU, an RU via its concept).

Ownership chain today: `users → topics`, `users → concepts → reinforcement_units → questions`, `users → notes | resources | study_sessions | quiz_sessions | user_settings`.

## Route audit

Status after #3 (questions), #4 (reinforcement units) and #29 (linked ids). "Owner in predicate" means the repository SQL includes the owner for every ID the route names.

| Route | Owner in predicate | Notes |
| --- | --- | --- |
| `GET/POST /api/topics`, `GET/PATCH/DELETE /api/topics/{id}` | yes | `topics.user_id` |
| `GET/POST /api/concepts`, `GET/PATCH/DELETE /api/concepts/{id}` | **yes, linked ids fixed in #29** | `topic_id`/`parent_id` verified owned on create and update via `repositories::owned`. |
| `GET/POST /api/reinforcement-units`, `GET/PATCH /api/reinforcement-units/{id}` | **yes, fixed in #4** | Was entirely unscoped: list with no filter returned every learner's RUs; get and update took any ID; create accepted any concept. Now joined through `concepts.user_id`; create is `INSERT ... SELECT` from the owned concept. |
| `GET /api/questions` | yes | join to `concepts.user_id` |
| `POST /api/questions/{id}/answer` | **yes, fixed in #3** | `find_owned`; RU read/update now also scoped by #4 |
| `GET/POST /api/notes`, `PATCH/DELETE /api/notes/{id}` | **yes, linked ids fixed in #29** | `concept_id`/`ru_id` verified owned on create. |
| `GET/POST /api/resources`, `POST /api/resources/upload`, `DELETE /api/resources/{id}`, `GET /api/resources/{id}/{content,pages,file}` | **yes, linked ids fixed in #29** | `topic_id`/`concept_ids` verified owned on both create paths. File serving and same-name overwrite are #11/#12. |
| `GET/POST /api/study-sessions`, `PATCH /api/study-sessions/{id}`, `POST .../complete` | **yes, linked ids fixed in #29** | `concept_ids` verified owned on create. |
| `POST /api/quiz-sessions`, `GET /api/quiz-sessions/{id}`, `POST .../submit`, `POST .../complete` | yes for the session | `submit` stores any `question_id` and never grades. Retired by the persisted-attempt slice; noted on #8. `list_answers(session_id)` is only reached after an owned session lookup. |
| `GET /api/progress` | yes | all CTEs filter on `user_id` |
| `GET/PATCH /api/settings` | yes | keyed by `user_id` |
| `GET /api/auth/me`, `POST /api/auth/{login,register,refresh,logout}` | n/a | identity itself; reviewed under #5/#6 |
| `POST /api/llm/*` | n/a | no owned records; client-supplied configuration is #6 |

### Repositories with unscoped methods but no route caller

| Repository | Method | Status |
| --- | --- | --- |
| `PgClaimRepository` | `find_by_id`, `list_by_concept`, `list_by_asset` | Dormant. Scope through `concepts.user_id` before any route uses them. |
| `PgEventRepository` | `list_by_aggregate` | Dormant. Filter on `events.user_id` before exposing. |
| `PgUserRepository` | `find_by_id`, `find_by_email` | Identity lookups, used only by auth. |

Row-level security as a database-enforced second layer is evaluated in #31, before Phase 2 providers are written.

## Adding a new domain

Copy the shape of `reinforcement_unit.rs`: every trait method takes `user_id`; every query joins to the owning table; creates that link to another record use `INSERT ... SELECT`; a test file in `crates/infra/tests/` seeds two learners and proves refusal. Add the route to the table above.
