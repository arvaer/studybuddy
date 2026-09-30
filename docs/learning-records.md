# Learning records

Schema landed by issue #8 (migration `20240105000000_learning_records.sql`); the recording operation by #9. Idempotent submission is #10 and authoring activities is #41. The existing quiz and question tables keep working until the frontend moves over (#13).

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

## Recording an attempt (#9)

`POST /api/attempts` with `{activityRevisionId, response, assistance?}`; `GET /api/attempts/{id}` returns the same receipt later. The receipt is `{attemptId, activityRevisionId, submittedAt, status, assessment}` where `status` is `pending`, `correct`, `partial` or `incorrect` and `assessment` is null while pending.

One repository method, `AttemptRepository::record`, owns the transaction:

1. Load the revision **through its activity's `user_id`**. Foreign or missing is `NotFound`, and nothing else runs.
2. Apply `domain::learning::assess` to the revision and the response. A string answer key with options is a `choice` assessment (the response must be one of the options, else 422). A string key without options is `exact_match`, compared after trimming, case-folding and collapsing whitespace. Any other key shape, or none, yields no assessment.
3. Insert the attempt with the response and the assistance list as given.
4. Insert assessment revision 1 if step 2 produced one.
5. Commit. A failure at any step leaves no attempt row.

The rule in step 2 lives only in the domain crate; the adapter calls it and must not grade in SQL. Model and manual assessment methods are reserved in the enum for later work and are not produced by this operation.

Tests: `backend/crates/infra/tests/learning_records_schema.rs` (schema) and `record_attempt.rs` (operation, including a forced failure after the attempt insert that proves rollback). The rule itself is unit-tested in `backend/crates/domain/src/learning.rs`.

Not in this operation: the learner's review schedule is not updated. The old `POST /api/questions/{id}/answer` still mutates RU state directly; retiring it is part of the frontend cutover (#13) and deletion workstream.
