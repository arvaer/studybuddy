# Learning records

Schema landed by issue #8 (migration `20240105000000_learning_records.sql`). The operation that writes these records is #9; idempotent submission is #10. Until those land, the existing quiz and question tables keep working and nothing reads these tables in production code.

## Tables

| Table | One row is | Mutable? |
| --- | --- | --- |
| `activities` | a stable, owned practice item: `user_id`, optional `concept_id`, `kind` (`recall`, `explain`, `apply`, `diagnose`) | yes (ownership and links only; no content) |
| `activity_revisions` | the exact content shown to the learner: `prompt`, `options`, `answer_key`, `rubric`, `source_resource_id`, `source_location`; numbered `revision` per activity | **no** |
| `attempts` | one submission against one revision: `response`, `assistance` used, optional `request_key`, `submitted_at` | **no** |
| `assessments` | one judgement of one attempt: `outcome` (`correct`, `partial`, `incorrect`), `method`, optional `score`, `feedback`; numbered `revision` per attempt | **no** |
| `attempt_status` (view) | every attempt with its explicit status: `pending` when no assessment exists, else the latest assessment's outcome | n/a |

Immutability is enforced by a `BEFORE UPDATE` trigger on the three record tables that raises an integrity-constraint error. Deletes are not blocked, so deleting a user cascades through everything they own.

## Rules the schema fixes

- **Editing is a new revision.** `UNIQUE (activity_id, revision)`. Attempts reference the revision they were made against, so an edit never changes what an earlier attempt answered.
- **Pending is not incorrect.** An attempt has no correctness column. Until an assessment row exists it is `pending`, and the view says so explicitly. This replaces the old `quiz_answers.is_correct = false` write.
- **A correction is a new assessment revision.** The original judgement stays; the view reports the latest one.
- **Request keys are per learner.** A partial unique index on `(user_id, request_key)` is the constraint #10 will use for idempotent submission.
- **Source link.** A revision may point at a `resources` row and a JSON `source_location`. When #12 introduces content-addressed artifacts, the revision gains an artifact address; nothing here needs to change for that.

## Ownership

`activities.user_id` is the root. `attempts.user_id` is denormalised so predicates stay one join short; the recording operation (#9) must verify it equals the activity's owner. The schema does not enforce that equality, and the ownership convention in [ownership.md](ownership.md) applies.

Tests: `backend/crates/infra/tests/learning_records_schema.rs`.
