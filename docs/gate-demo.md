# Phase 1 gate: one authored, source-linked activity end to end

The gate in [hardening-plan.md](hardening-plan.md) ("Gate for beginning application integration"; issue #16), run on 2026-10-01 against `main` at the merge of #49 plus the one fix below. It is a bounded demonstration against the real HTTP API and the real frontend, without a live operator. It does not claim that scheduling, exports or generated widgets exist.

## How to run it

```sh
scripts/dev-db.sh up          # disposable database; nothing else is touched
(cd backend && cargo build)
scripts/gate-demo.sh          # builds, starts a backend on :3100 with a scratch UPLOADS_DIR, runs, stops it
```

The script signs up two throwaway learners, authors one `recall` activity whose revision cites an uploaded `notes.txt`, and drives every gate step over HTTP with `curl` and `jq`, printing one `PASS`/`FAIL` line per check. Step 4 kills and restarts the backend process between the submission and the re-read. `GATE_BASE=http://127.0.0.1:3000 scripts/gate-demo.sh` runs the same checks against a backend you started yourself, skipping the restart. Rows it creates stay in the development database; it deletes nothing.

The browser half (sign in, select, submit, refresh, open the source) was done by hand in the frontend dev server against the same database and is recorded below. Run it yourself with `cd frontend && npm run dev` and a backend on `:3000` with `COOKIE_SECURE=false` and a fixed `JWT_SECRET`.

## Evidence per step

| Step | API check (`scripts/gate-demo.sh`) | Browser | Regression test |
| --- | --- | --- | --- |
| 1. Sign in, select the activity, load its exact revision and source reference | Login; `GET /api/activities` lists the activity with its current revision id; `GET /api/revisions/{id}` carries `sourceResourceId`, `sourceArtifactId` and `hasAnswerKey: true` with no `answerKey` field | Quiz page shows "Activity 2 of 2 · recall · revision 1", the prompt, three options, and "Source: open the cited version" linking to `/api/artifacts/{id}/bytes` | `authoring.rs::create_writes_the_activity_and_revision_one_together` |
| 2. Submit; ownership validated; original attempt and assistance recorded | `POST /api/attempts` with a request key, the response and one `assistance` entry answers 201 with the same revision id and the response as submitted | Typed an answer on the `explain` activity and pressed Submit; card switched to the recorded phase | `record_attempt.rs::keyed_activity_is_assessed_in_the_same_operation`, `unkeyed_activity_stays_pending_with_assistance_recorded` |
| 3. Deterministic assessment, or explicit pending | Keyed `recall`: `status: correct`, `assessment.method: choice`. Unkeyed `explain`: `status: pending`, `assessment: null` | Card shows "Recorded, awaiting assessment" for the `explain` answer | `domain::learning` unit tests; `record_attempt.rs::choice_activity_wrong_option_is_incorrect` |
| 4. Refresh the page and restart the backend; activity and attempt remain | Backend process killed and restarted with the same database; fresh login; `GET /api/attempts/{id}` returns the same attempt id, revision, response and status; `GET /api/activities/{id}` still names the same current revision | Backend restarted (same `JWT_SECRET`), quiz page hard-loaded: the answer text, "Recorded, awaiting assessment" and "(already recorded)" render from the backend receipt | `attempt-state.test.ts` (restore from stored attempt id), `activity-card.test.tsx` |
| 5. Retry returns the existing result; conflicting reuse adds nothing | Same key and payload: 200 with the same attempt id. Same key, response changed: 409; the original attempt re-read unchanged | Not repeated in the browser; the card reuses its key on retry (tested) | `idempotent_submission.rs` (replay, conflict, per-learner scope, twelve concurrent duplicates) |
| 6. Second learner refused without exposure or mutation | As learner B: revision, activity, attempt and artifact bytes all 404; submitting against A's revision (even with A's request key) 404; adding a revision 404; B's activity list excludes A's. A's activity still at revision 1, attempt still `correct` | n/a | `authoring.rs::second_learner_is_refused_everywhere_and_writes_nothing`, `record_attempt.rs::foreign_revision_is_not_found_and_nothing_is_written`, `artifacts.rs::artifacts_are_owner_scoped_and_immutable` |
| 7. The cited source version survives a later same-name upload | Second `notes.txt` with different bytes gets a new `artifactId`; the revision still cites the first; `GET /api/artifacts/{first}/bytes` returns the original line | Opened "open the cited version" after the second upload: the original text is served | `artifacts.rs::a_revision_pins_the_source_version_it_was_authored_against`, `same_name_upload_is_a_new_version_and_the_earlier_one_is_untouched` |

Builds and tests at the time of the run: backend `cargo build --workspace --all-targets` clean, 89 backend tests passing; frontend `tsc`, `eslint`, 27 vitest tests and `npm run build` clean.

## What the run found

**A hard refresh of any protected page bounced to the login form** (fixed in this change). `ProtectedRoute` redirected to `/auth` whenever `user` was null, which it is while `GET /api/auth/me` is still restoring the session from the cookie. The redirect happened before the response arrived, so step 4 failed in the real UI even though the cookie was valid and every API check passed. The guard now renders nothing while `isLoading` is true and redirects only once restore has finished without a user. `frontend/src/components/ProtectedRoute.test.tsx` pins the three states. This did not show up before because the frontend's own tests never mounted the guard and the Quiz page was always reached by client-side navigation after login.

**A backend started with a different `UPLOADS_DIR` than the one that took the upload answers 500 on the bytes**, with the log naming the artifact and its address (`blob missing at <sha256>`), and metadata still answers. That is the documented "row exists, blob missing" case in [artifacts.md](artifacts.md), not a gate failure: copying the blob to its address under the new directory healed it with no catalog change, which is also the documented restore procedure. The script avoids the situation by giving its backend one scratch directory for the whole run.

**Restarting with a new `JWT_SECRET` signs every browser out.** Expected (access tokens are stateless HS256), but worth stating for anyone reproducing step 4: keep the secret fixed across the restart, as a deployment would.

## Not covered

- Assistance is recorded as given; nothing produces it yet. The AI chat sheet on the quiz page is still a mock until Phase 2.
- Sessions still end 15 minutes after login (#35). The run finished within that window.
- Row-level security is an open evaluation (#31), not a gate requirement.
