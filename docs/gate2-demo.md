# Phase 2 gate: the operator over a long horizon

The gate in [phase-2-build.md](phase-2-build.md) (20d-ii; issue #20), run on 2026-10-03 against `main` at the merge of #71, with the real backend, the disposable PostgreSQL from `scripts/dev-db.sh`, and the real model (`claude-opus-5-5` through upstream's adapter, keyed by `ANTHROPIC_API_KEY` in `backend/.env`). It demonstrates the gate 2 ruling: stop and start, a kill mid-think, days passing, the same plan continuing. It does not claim gate 3 (several goals, clock re-tests), hints, or any assessment quality.

## How to run it

```sh
scripts/dev-db.sh up            # disposable database; nothing else is touched
scripts/operator-demo.sh        # builds, starts a backend on :3200, runs the four steps, stops it
```

The learner is `demo@demo.test` with password `demodemo` (`DEMO_EMAIL`, `DEMO_PASSWORD`), signed up on first use, so the frontend can be opened on that account afterwards to see what the run built. A workspace takes one goal, so a later run on a database where that account already has one uses `demo+1@demo.test`, then `+2`, with the same password, and prints which. About five model calls, billed to the key. Rows stay in the development database; nothing is deleted. The script prints one `PASS`/`FAIL` line per check and the prompt the operator published at each step, and names the backend log at the end.

## The run

| Step | What happened | Check |
| --- | --- | --- |
| 1. Intent in | `POST /api/workspaces/{id}/goal` answered 202 `thinking`. 4 s later the workspace was `waiting` on the first activity: a prompt asking what the return G_t is and why maximise it rather than the next reward. | `PASS` ×2 |
| 2. Answer wakes the operator | `POST /api/attempts` answered 201. 7 s later a different activity was waiting: a worked γ = 0.5 comparison of two reward paths. The log shows the attempt receipted under the wait's effect id and the run going on. | `PASS` ×2 |
| 3. Kill mid-think | The second answer went in, and one second into the model call the backend got `SIGKILL`. Restarted, fresh login. The first successful workspace read allowed the interrupted `call/model` park again (log: `interrupted think allowed again … family=call/model`), the model answered, the third activity landed, exactly one new activity. Total 52 s, of which about 48 s was the dead process's lease (below). | `PASS` ×5 |
| 4. Days pass | Backend stopped cleanly. The one time-bearing row, `workspace_sessions.lease_until`, moved back three days. Restarted, fresh login: the same third activity waiting, the same goal, no settle logged, the activity count unchanged, so reopen asked no model and published nothing. The answer to it was accepted and the operator went on to the next activity in 5 s. | `PASS` ×4 |

Backend tests at the time of the run: 125 passing.

## What the run found

Both are UX errors the learner would meet in the first session. Neither was fixed in this change; both were, the same day (20e in #75, 20f as grading through the capsule).

**After a kill, the page is dead for up to a minute.** The dead process still holds the workspace lease (TTL 60 s, renewed every 20 s), so the restarted process gets `OperatorError::Leased` on every workspace read until it lapses. That surfaces as HTTP 500 "the operator is unavailable", and because the quiz page loads the current workspace in the same `Promise.all` as topics and activities, the whole page shows a load error, not just the operator line. In the run: 48 consecutive 500s, one per second, then recovery. Two fixes, either enough on its own, both worth doing: `GET /api/workspaces/{id}` should answer the goal with an `unavailable` operator state instead of failing, so the page renders and polls; and the lease should be short (10 s, renewed every 3 s) or stealable by a process on the same host that can see the old pid is gone. The plan already says one process holds every lease, so a short TTL costs nothing.

**Every free-text answer is marked "Not quite right".** The operator publishes `recall` activities whose answer key is a model sentence ("The return G_t is the total cumulative future reward from time t on…"). The manual route assesses a keyed activity without options by `exact_match`, so each of the three attempts was `incorrect` with a red cross, though two were fine answers the coach built on. The operator reads the attempt itself and does not use the assessment, so the loop is unharmed, but the learner is told they were wrong every time. The honest state today is `pending`: the present endpoint should keep the model's expected answer off `answer_key` for an activity without options (or the capsule should publish them as `explain`), until the Phase 3 "assessed explanations" slice gives `method = model` a real path.

Two smaller things, for the record. The coach re-asked the γ = 0.5 computation after an answer that talked about discounting in general rather than computing it, which is the intended behaviour ("go deeper where the attempt was weak") and reads well. And each think took 4–7 s with the default model, so the two-second poll is the right cadence.

## Not covered

Gate 3: several goals in one workspace, the clock-source re-tests, the decide family. Hints (20c). The browser half of this gate was not repeated by hand; the frontend's behaviour during the lease window is read from `Quiz.tsx` and the 500s in the log, not observed.
