# Learning records

Schema landed by issue #8 (migration `20240105000000_learning_records.sql`); the recording operation by #9; idempotent submission by request key by #10; authoring by #41. The existing quiz and question tables keep working until the frontend moves over (#13).

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
- **Request keys are per learner.** A partial unique index on `(user_id, request_key)` is the constraint idempotent submission (#10) relies on.
- **Source link.** A revision may point at a `resources` row and a JSON `source_location`. Since #11 it also carries `source_artifact_id`, the exact upload the resource had when the revision was authored; see [artifacts.md](artifacts.md).

## Ownership

`activities.user_id` is the root. `attempts.user_id` is denormalised so predicates stay one join short; the recording operation (#9) must verify it equals the activity's owner. The schema does not enforce that equality, and the ownership convention in [ownership.md](ownership.md) applies.

## Authoring (#41)

| Route | Does |
| --- | --- |
| `POST /api/activities` | `{kind, conceptId?, revision: {prompt, options?, answerKey?, rubric?, sourceResourceId?, sourceLocation?}}` creates the activity and revision 1 in one transaction. 201. |
| `POST /api/activities/{id}/revisions` | Same revision body; adds the next revision to an owned activity. 201. |
| `GET /api/activities` | The learner's activities, newest first, each with its `current` (highest-numbered) revision. |
| `GET /api/activities/{id}` | One activity with its current revision. |
| `GET /api/revisions/{id}` | One exact revision, the thing an attempt cites. |

Content rules (`RevisionContent::validate` in the domain crate): the prompt is required and at most 10 000 characters; options, if given, are at least two distinct non-blank strings and the answer key must then be one of them; a source location needs a source resource. `kind` is `recall`, `explain`, `apply` or `diagnose`.

**The answer key and rubric are write-only.** Read responses carry `hasAnswerKey` instead, so the client answering an item never receives its key. The linked concept and source resource must be the caller's; a foreign one is `NotFound` before any row is written.

Revising locks the activity row (`SELECT ... FOR UPDATE`) and then reads `max(revision) + 1` in a **separate statement**. Under READ COMMITTED the snapshot is taken before the lock wait, so a `max()` inside the locking statement reads a stale count; the concurrency test in `authoring.rs` caught exactly that. Earlier revisions are never touched, and attempts keep pointing at the revision they answered.

## Recording an attempt (#9)

`POST /api/attempts` with `{requestKey, activityRevisionId, response, assistance?}`; `GET /api/attempts/{id}` returns the same receipt later. The receipt is `{attemptId, activityRevisionId, submittedAt, status, assessment}` where `status` is `pending`, `correct`, `partial` or `incorrect` and `assessment` is null while pending.

One repository method, `AttemptRepository::record`, owns the transaction:

1. Look up `(user_id, request_key)`. If it exists, the replay rule below answers and nothing else runs.
1. Load the revision **through its activity's `user_id`**. Foreign or missing is `NotFound`, and nothing else runs.
2. Apply `domain::learning::assess` to the revision and the response. A string answer key with options is a `choice` assessment (the response must be one of the options, else 422). A string key without options is `exact_match`, compared after trimming, case-folding and collapsing whitespace. Any other key shape, or none, yields no assessment.
3. Insert the attempt with the request key, the response and the assistance list as given, `ON CONFLICT DO NOTHING` on the request-key index.
4. Insert assessment revision 1 if step 2 produced one.
5. Commit. A failure at any step leaves no attempt row.

## Idempotent submission (#10)

Every submission carries a `requestKey` (1 to 128 characters; a client-generated UUID is the intended shape), unique per learner. Resending it is safe:

- **Same key, same payload** (revision, response and assistance all structurally equal): the existing receipt is returned with `200` instead of `201`, and no row is written.
- **Same key, different payload**: `409 Conflict`. The original attempt is not altered and nothing is written.
- **Concurrent duplicates**: the insert uses `ON CONFLICT DO NOTHING` on the `(user_id, request_key)` index, so the second writer waits for the first to commit, gets no row back, rolls back, and replays through the same rule. Exactly one attempt exists afterwards.

The payload rule is `RecordAttempt::same_payload` in the domain crate. The replay read runs on the connection the transaction already holds; acquiring a second one there would exhaust the pool under concurrent duplicates, which the concurrency test caught.

The rule in step 2 lives only in the domain crate; the adapter calls it and must not grade in SQL. Model and manual assessment methods are reserved in the enum for later work and are not produced by this operation.

Tests: `backend/crates/infra/tests/learning_records_schema.rs` (schema), `authoring.rs` (create, revise, two learners, validation, eight concurrent revisions), `record_attempt.rs` (operation, including a forced failure after the attempt insert that proves rollback) and `idempotent_submission.rs` (replay, conflict, per-learner scope, key validation, twelve concurrent duplicates). The rule itself is unit-tested in `backend/crates/domain/src/learning.rs`.

Not in this operation: the learner's review schedule is not updated. The old `POST /api/questions/{id}/answer` still mutates RU state directly; retiring it is part of the frontend cutover (#13) and deletion workstream.

## Frontend (#13)

The quiz page (`frontend/src/pages/Quiz.tsx`) walks the learner's activities and answers each through `POST /api/attempts`; it no longer grades, and no answer key reaches the browser. The pieces:

- `frontend/src/lib/activities.ts` — the activities, revisions and attempts API as the backend sends it. `recordAttempt` reports `replayed` from the status code (200 replay, 201 new).
- `frontend/src/lib/attempt-state.ts` — one attempt's client state, pure and tested without React. Phases are `restoring`, `draft`, `submitting`, `accepted` (backend state) and `failed` (with `retryable`). Per revision the browser keeps only the unsent draft, the request key with the exact payload it was minted for, and the attempt id once accepted, under `localStorage` key `studybuddy.attempt.<revisionId>`.
- `frontend/src/hooks/use-attempt.ts` and `frontend/src/components/activity-card.tsx` — the hook and the card that renders exactly one phase.

Rules the tests pin (`attempt-state.test.ts`, `activity-card.test.tsx`):

- **Retry reuses the key.** A network failure or 5xx keeps the key and payload; the retry replays and the backend records once. A changed answer gets a new key, since a reused key with a different payload is a 409.
- **A 4xx is not retryable.** The card shows the rejection and the draft stays editable.
- **Pending is not wrong.** A receipt without an assessment renders as "recorded, awaiting assessment".
- **Refresh restores backend state.** On mount, a stored attempt id is re-read with `GET /api/attempts/{id}`; the receipt now carries `response`, so what was answered is shown from the backend, not from a local copy. A 404 (a different learner on this browser, or a revision gone) drops the local record.
- **Drafts survive a refresh** and are labelled as unsent.

Gate step 4 (refresh the page and restart the backend; the accepted activity and attempt remain) is therefore: the activity list, the revision and the attempt receipt all come from the database on load, and the only local state is the draft and the index of what was recorded.


## What the UI no longer claims (#14)

No data or policy backs a due date, a mastery judgement or a study streak, so the UI does not assert them:

- Reinforcement-unit states are shown by their stored name. `stable` is labelled "Stable", not "Mastered", and topic and concept cards count units that are stable, not a mastery percentage.
- The dashboard shows concept and unit counts only. The streak and study-time cards are gone: they were computed from `study_sessions`, which the quiz stopped writing when it moved to attempts (#13).
- The sidebar no longer shows a hard-coded streak, and settings no longer offer streak alerts.
- The quiz no longer opens a configuration modal. Its spaced-repetition settings, due counts and "include mastered cards" switch had no effect on which activities were served.

Dead after this change, noted for the deletion ticket (#15): `frontend/src/components/quiz-config-modal.tsx`; the `QuizSessionConfig`, `SRSSettings`, `CramSettings`, `CardPriority` and `defaultQuizConfig` definitions in `frontend/src/types/study.ts`; `streakDays` and `totalStudyTime` on `LearnerProgress` and in the `/api/progress` query; `streakAlerts` on settings.
