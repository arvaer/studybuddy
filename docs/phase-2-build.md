# Phase 2 build plan: the operator loop

October 1, 2026. Scopes the Phase 2 tickets (#18, #19, #20) and the first Phase 3 slices that the operator UX ruling and the guided-reading design call for. Decisions it rests on: [capsule-integration.md](capsule-integration.md), "Revisit after Phase 1" and "Learner UX for Phase 2"; the product in [operator-design.md](operator-design.md); the pedagogy in [learning-design.md](learning-design.md). Capsule pin `4b77dcc`; the working constraints of the hardening plan apply to every ticket.

## Why this exists (owner, 2026-10-01)

Two purposes, both real, no revenue. First, **StudyBuddy is the demonstration of Capsule Core**: that this kind of long-running, source-bound, recoverable agent work is built with relative ease from small capsule blocks. Second, **the owner is the first learner**, using it to become an RL expert for the Neural Routing thesis. The two keep each other honest: the demo is only convincing if the learning is real, and the learning only happens if the loop actually runs.

Consequences for every ticket below:

- **The capsule source is the product.** Each block (present, wait, hint, locate, critique, assess, plan) is a capsule or verb that fits on one screen and reads as what it does. Legibility of `backend/capsules/*.capsule` and the environment template outranks frontend polish.
- **The gate demos are the deliverable.** `gate-demo.md` and its script are the pattern: a narrated run anyone can execute that shows a crash and a recovery, a refused scope, a replay that asks no provider. Gate 2 and gate 3 are written for a reader who wants to see what Core does, not a user who wants a nicer page.
- **Frontend stays minimal on purpose.** The quiz page, an intent box, a hint control, a span beside the question. Nothing that does not show a Core capability or serve the owner's own study.
- **Real sources, real study.** The first workspace is the owner's RL reading list; the gate 2 and gate 3 runs use those uploads, so every demo doubles as a study session.

## What gates 2 and 3 must show (owner, 2026-10-01)

The owner's reading of the demo: Core's case is **learning tracked over a long horizon**, which means stopping and starting, switching between several tasks, and keeping structure across all of it. Context management is the biggest unsolved part and the owner holds no opinion on how it should work, for lack of data. So:

- **Gate 2 is stop and start.** One goal; the learner leaves mid-activity, the backend is killed and restarted, days pass by the clock, and the next wake continues the same plan with nothing lost and nothing published twice. The record, not a context window, is what carries the plan across the gap.
- **Gate 3 is many threads, one structure.** Two or three goals in one workspace, interleaved by the learner, each with its own plan and progress, re-tests landing from the clock in the middle of other work, and a reader anyone can run that shows which goal each published activity served and why it was chosen. This is the Level 4 demo: planner, delegated children, timed returns, all in small blocks.
- **Context management is instrumented, not decided.** Every operator wake records what it was handed (goal revision, open activities, recent attempts, hints, elapsed time) as a node beside its decision. Phase 2 and 3 then accumulate the data a policy needs; the policy itself is a Phase 3 ticket written after the owner has read the records. No summarisation, forgetting or retrieval scheme is chosen in this plan. The primitives this needs upstream (purpose as the frame's goal with a `done` predicate, a frame inspecting its own record, notes citing addresses) were proposed in [capsule-corp #306](https://github.com/Prominent-Systems/capsule-corp/issues/306) and triaged on 2026-10-02: a frame reading its own record is ticketed upstream as K1 (ruling) and K2 (build), after the launcher; purpose as the frame's goal is accepted as a reading; a `done` predicate is deferred; notes are an ordinary provider, and a note read back is untrusted. Views will be kernel vocabulary (parks, children, effects), bounded and charged, and over descendants structure only, so "open under this goal" is computed in the learning capsule with list primitives, not asked of the kernel. Activation, the full K-line, stays deferred until records exist.

## What ships at the end of Phase 2

A learner types one intent into an empty workspace and is answering a source-cited activity within one screen. Each accepted attempt wakes the operator, which publishes the next activity without being asked. A process kill at any point reopens with nothing lost and nothing published twice. The frontend is the existing quiz page plus an intent box and a hint control. No scheduling policy, mastery judgement, export, generated interface or code execution.

## The one architectural choice: embed now, launcher-ready

Upstream's **L1 launcher** (one binary: environment file, capsule refs, a connector manifest binding families to providers, opens the session on storage, runs the owner's loop) is planned and not started. Waiting for it would idle Phase 2 on an unknown date, and its connector types are not yet defined, so we cannot target them.

We **embed** `capsule_host::owner::Owner` in the backend process now, and we shape every provider as a **thin HTTP endpoint on the backend** rather than a Rust closure that reaches into services directly:

```text
POST /internal/effects/{workspace}/{family}
Authorization: Bearer <OPERATOR_SECRET>        Idempotency-Key: <effect id>
body:  {"id": "sha256:…", "capability": "learning/present", "payload": [path, kind, prompt, answer-key]}
reply: {"value": …} | {"refused": "why"} | {"declined": "why"} | {"unknown": "why"}
```

The body and the reply are upstream's connector protocol as ruled in L1c (sdk-surface, "The connector protocol", 2026-10-01), so the launcher's `http` connector fits with no endpoint change. Idempotent by effect id (the receipt table), authenticated by a process-local secret, bound to a workspace by the URL, which the session's manifest fixes, with the payload's scope path required to lie inside that workspace; never reachable from the browser. The embedded owner's providers for the two effects that touch the database are one-line clients of these endpoints over loopback; the model is upstream's Claude adapter installed in-process on the owner thread, which is what the launcher's own `claude` connector does. When L1 ships with any HTTP-shaped connector, the launcher replaces the embedded owner and **no provider changes**; if L1 never fits, nothing was lost but one hop on loopback. This is the only place Phase 2 bets on upstream's direction, and the bet is hedged.

```mermaid
flowchart LR
  UI[Quiz page + intent box] --> API[Axum routes]
  API --> PG[(PostgreSQL: app tables, receipts, leases, capsule schema)]
  API -->|Handle| Owner[Owner thread: Session on PgStorage]
  Owner -->|effect| EP[/internal/effects/*/]
  Owner -->|call/model| Claude[Claude adapter, in-process]
  EP --> SVC[ActivityService / AttemptService]
  SVC --> PG
  Owner -->|nodes, refs| PG
```

## Tickets, in order

Each is one PR, small, with a reading-order guide. Sizes: S under 200 lines, M under 500, L needs a split.

| # | Ticket | Size | Lands live |
| --- | --- | --- | --- |
| 18a | **Pin and schema.** Probe pin to `4b77dcc`, lockfile; migration for the `capsule` schema (`nodes`, `refs`) exactly as `PgStorage` expects, plus `effect_receipts (effect_id PK, workspace_id, family, payload JSONB, recorded_at)` and `workspace_sessions (workspace_id PK, session_name, owner_lease UUID, lease_until)`. | S | Tables exist; probe green on the new pin. |
| 18b | **Open and reopen on Postgres.** A `capsule_host` dependency with the `postgres` feature; an `OperatorHost` module that spawns one `Owner` per workspace on demand, opening `PgStorage::existing` on the owner thread with its own pool, holding the lease row. Test: open, run the probe's capsule with scripted providers, kill the owner, reopen from the database, same record. | M | A session survives a restart on the real database. |
| 18c | **Receipts and the four crash points.** `complete` by effect id from the receipt table on reopen. Test per crash point: before dispatch (deny), during possible delivery (`Unknown` then complete from receipt), after the domain commit, after the reply is recorded (`Already`). Scripted providers, real Postgres. | M | Recovery proven where it will run. |
| 19a | **Effect endpoint.** `/internal/effects/{workspace}/learning.present` calling `ActivityService::create` inside one transaction with the receipt insert; process-local auth; workspace binding. Test: the manual route and the endpoint write identical rows for the same input; a replayed effect id returns the stored receipt and writes nothing. `learner.attempt` moved to 20b, where its caller first exists (owner, 2026-10-03). | M | The publication provider exists and is idempotent. |
| 19b | **The model provider.** Upstream's `capsule_host::claude::Claude` installed in-process on the owner thread as `call/model`, as the launcher's own `claude` connector is: real tool calling, thinking kept across turns, a refused conversation declined. Keyed by `ANTHROPIC_API_KEY`, the variable capsule-corp reads; `OPERATOR_MODEL` defaults to `claude-opus-5-5`. No endpoint and no receipt: the model has no side effect outside the record, so a park on it after a crash is allowed again under the same id (H3). Supersedes the `/internal/effects/…/model.call` endpoint (owner, 2026-10-03: "run the claude adapter in process"). | S | The capsule can think. |
| 19c | **Learning capsule v1.** `backend/capsules/learning.capsule` and `learning.environment`, templates with one hole, `{{workspace}}`, in the scope paths and the grants; `operator::capsule` fills and compiles them per workspace and checks them at startup, logging the definition address. The model's verbs are `coach/present` (publish one activity at `workspaces/<id>/activities`, then `learner/wait` for the attempt) and `coach/finish`; follow-ups are the model presenting again after reading the attempt. A capsule filled for another workspace is refused at admission. The `learning/present` payload is `[path kind prompt answer-key]`, built into the manual route's DTO at the endpoint, since capsule source has lists and strings, not objects. | S | The program the operator runs is in the repo. |
| 20a | **Goal and start.** `GET /api/workspaces/current` makes the learner's workspace on first sight. `POST /api/workspaces/{id}/goal` stores the intent as goal revision 1 (one per workspace in Phase 2) and starts the run in the background (`run_once` keyed by the goal revision id); 202 with the workspace `thinking`. `GET /api/workspaces/{id}` returns goal, operator state (`thinking`, `waiting` with `currentActivityId`, `idle` with how the last run ended, `stalled` with what it is parked on). `OperatorRuntime` opens a workspace's session on demand, reconciles on open, installs the model and a loopback client of `learning.present`; `OPERATOR_SECRET` defaults to a per-process secret. Frontend: the intent box on an empty workspace, a status line polled while thinking, nothing else. | M | Intent in, first activity out. |
| 20b | **Answer wakes the operator.** On an accepted attempt against an operator-published revision, the route completes the `learner.wait` park by effect id from the attempt receipt; the owner runs on; the next `present` publishes. Frontend: the quiz page polls workspace state and shows the new activity. | M | The loop closes. |
| 20c | **Hint.** `POST /api/attempts/drafts/{revisionId}/hint` asks the operator (a `learner.hint` effect served by the model provider), returns text, and the attempt that follows records it as assistance. Frontend: one control on the card, labelled. | S | Assistance exists and is recorded. |
| 20d | **Gate 2.** `scripts/operator-demo.sh`: intent, first activity, answer, follow-up, kill the backend mid-think, restart, same follow-up, no duplicate; then leave mid-activity, advance the clock by days, return, same plan continues. Doc like `gate-demo.md`. | S | Phase 2 is demonstrated, not claimed. |

Order: 18a → 18b → 18c → 19a → 19b → 19c → 20a → 20b → 20c → 20d. 19a can start after 18a since it needs only the receipt table.

## First Phase 3 slices this unlocks

Filed now so the Phase 2 work keeps their data in reach; built after 20d.

| Ticket | Slice |
| --- | --- |
| **Source spans** | A revision's `source_location` becomes `{page, start, end}` the quiz page renders beside the question from `/api/resources/{id}/pages`, so the learner reads the passage inside the activity. Pretests and gated passages are then just activities. |
| **Source navigator** | At upload, derive a structure tree (headings with page ranges) from the page text and store it on the resource. A read-only `source.locate` effect walks the tree with the model and returns page ranges, the vectorless-RAG pattern; the operator cites them. Shaped as upstream's reader child once delegation is exercised. |
| **Assessed explanations** | `assessments.method = model` for `explain` activities, produced by the operator after the attempt, never by the publication path; pending until then. |
| **Wake context, recorded** | Each wake's handed-in context becomes a node the gate reader prints beside the operator's decision, so the context policy is chosen from records rather than opinion. The first consumer is gate 3. |
| **Delayed re-tests** | Items answered with assistance or recently return unaided at preselected times via the clock-source park and timekeeper (T1, T2). |

## Not in this plan

Row-level security (#31, deferred). Mastery, due dates, streaks (#14 stands). Exports. Generated interfaces. Web research. Code execution. Multiple owners per session or multi-process deployment: one backend process holds every lease; the lease row is what makes a second process refuse, not scale.

## Risks, each with its tell

- **L1 lands with a connector shape the endpoints do not fit.** Tell: its manifest binds providers by something other than a URL. Cost: an adapter binary, providers unchanged.
- **`PgStorage` on the owner thread contends with the app pool.** Tell: lock waits on `capsule.refs`. Cost: none expected; separate pool, separate tables, one writer.
- **The model provider is slow and the HTTP request waits on it.** Prevented by design: routes return workspace state, never a run result; the owner thinks off the request path.
- **Upstream moves the SDK again.** Tell: the probe fails on a pin bump. Cost: known, since the probe is our canary and runs in CI.
