# StudyBuddy hardening before Capsule integration

September 29, 2026. Agreed sequence: clean up and harden StudyBuddy first, then integrate Capsule for the learning loop and personal operator. This plan supersedes the implementation order in the earlier preparation and integration notes. The existing Capsule compatibility probe remains an isolated experiment.

Product direction remains the [personal operator](operator-design.md). Architecture references: [persistence](persistence-design.md), [learning design](learning-design.md), and [Capsule integration assessment](capsule-integration.md).

## Phase 1: a dependable application foundation

Keep Axum, PostgreSQL/SQLx, useful React/reader components, and the domain/application/infrastructure boundaries. Build a small, persisted learning flow and remove obsolete paths as their replacements become usable. Avoid investing in new behavior on legacy grading and scheduling paths that this plan retires.

The findings below were confirmed by source inspection at StudyBuddy `3900c026ec9020c60261f603d9dfde84fc900fd3`, with local design/probe files present. They are not results of a live penetration test or a complete audit. Application builds, migration execution, and end-to-end checks have not been run as part of this planning update.

| Workstream | Inspected starting point | Completion evidence |
| --- | --- | --- |
| Development baseline | [Backend manifest](../backend/Cargo.toml), [frontend scripts](../frontend/package.json); [example test](../frontend/src/test/example.test.ts) only asserts `true` | Document and execute fresh setup, migrations in an isolated database, builds, and meaningful tests for changed behavior. Record pre-existing failures separately. |
| Identity and ownership | [Answer route](../backend/src/routes/questions.rs) discards `AuthUser`; [question lookup](../backend/crates/infra/src/repositories/question.rs) selects by ID without ownership | Carry authenticated identity through application commands and database predicates. Tests prove learner A cannot read or change learner B's records, including through linked IDs. |
| Configuration and model access | [Startup](../backend/src/main.rs) falls back to a known JWT secret; [LLM configuration](../frontend/src/lib/llm.ts) lives in the browser and the [proxy](../backend/src/routes/llm.rs) accepts client configuration | Fail on missing required secrets; define server-owned model configuration, allowed outbound destinations, timeouts, and error handling before enabling hosted model access. Review authentication/session behavior within this workstream. |
| Persisted learning records | [Quiz UI](../frontend/src/pages/Quiz.tsx) grades locally; [quiz service](../backend/crates/app/src/services/quiz.rs) writes `false`; [answer service](../backend/crates/app/src/services/question.rs) separately grades and mutates an RU | One application operation records an immutable attempt against an owned activity revision. Assessment is explicit; pending is not incorrect. Related updates commit together. Same request key/payload returns its receipt; conflicting reuse is rejected. |
| Sources and artifacts | [Upload service](../backend/crates/app/src/services/upload.rs) writes under user/filename, allowing same-name replacement | Preserve source bytes and version identity with an application artifact store, ownership checks, metadata, and source references. Verify same-name uploads cannot change earlier versions. Define file/DB failure and backup behavior. |
| Frontend behavior | Current quiz progress and feedback are React state; due counts are based on RU status | Render accepted backend state, retain drafts appropriately, expose loading/pending/retry states, and restore the same accepted activity/attempt after refresh. Remove unsupported due/mastery claims until their data and policy exist. |
| Targeted deletion | Duplicate grading paths and content/scheduling fields overlap | Remove superseded endpoints, state, and dependencies after replacement consumers are verified. Preserve existing migrations/data until migration or clean-database cutover is deliberately chosen. |

These are workstreams, not one large implementation diff. Establish the build/test baseline first, then address ownership and configuration, then the smallest coherent persisted learning flow and its source/UI support. Deletion accompanies replacement rather than becoming an unrelated rewrite.

## First proposed correctness slice

Close the ownership gap in `POST /api/questions/{id}/answer`: propagate the authenticated user through the application operation and restrict the question lookup through its RU/concept ownership relation before any review-state mutation. Align callers and repository contracts so a request cannot bypass the check through the service boundary.

The regression checks should show an owned question still works and a foreign/nonexistent question is rejected without changing any learner's review state. If SQLx metadata changes, regenerate and verify it against the isolated schema. This patch does not attempt to redesign grading or scheduling; those paths are replaced by the subsequent persisted-attempt slice.

No application or test patch is included in this planning update. Implementation follows the owner's small-diff, explanation, and approval checkpoint.

## Gate for beginning application integration

Demonstrate one authored, source-linked learning activity without a live operator:

1. Sign in, select the activity, and load its exact revision and source reference.
2. Submit an answer; the backend validates ownership and records the original attempt and any assistance.
3. Produce a deterministic assessment where appropriate, or retain an explicit pending state.
4. Refresh the page and restart the backend; the accepted activity and attempt remain available.
5. Retry the same submission and get the existing result; conflicting reuse does not add another attempt or alter the original.
6. Attempt access as a second learner; the operation is refused without exposing or mutating the first learner's records.
7. Open the source version cited by the activity and verify that a later same-name upload did not replace it.

Backend/frontend builds and relevant regression tests must pass. This is a bounded gate, not a claim that all future product features or production-scale operations are finished. A complete scheduling engine, all exports, and generated widgets are not prerequisites for the first Capsule learning loop.

## Phase 2: Capsule owns the agentic learning loop

Revisit the supported SDK after Phase 1. Track [SDK recovery bug #284](https://github.com/Prominent-Systems/capsule-corp/issues/284) and [async hosting feature request #287](https://github.com/Prominent-Systems/capsule-corp/issues/287). These issues inform adapter choices; they do not block application hardening.

Implement Capsule node/ref persistence and the chosen worker/async boundary, then turn the Phase 1 application operations into providers. The manual UI and the operator must share ownership checks, revision rules, transactions, and receipts. Avoid a second grading or publication implementation inside providers.

Connect authenticated start/answer/status operations to a scoped capsule. Replace the probe's scripted model with a server-configured adapter; publish an activity, park for the learner, resume after an accepted attempt, and publish a follow-up. Verify reopen, unknown-delivery reconciliation, and the domain-commit/core-reply failure window against the real storage adapter.

The [compatibility probe](../experiments/capsule-operator/README.md) establishes a starting SDK boundary, not completed app integration. Keep it isolated until this phase.

## Phase 3: richer personal workspaces

Build on the working loop: source research, citable notes, experiment/code artifacts, default Anki exports, and versioned interface composition. The operator can arrange supplied React blocks first; generated HTML/SVG/custom components need their rendering and action contracts, and generated executable tools need the separately reviewed confinement/promotion path.

Evaluate the RL thesis and exam-preparation journeys against their distinct goals and evidence. Adaptation, grading quality, scheduling, and delayed learning outcomes require evaluation beyond successful execution of the operator loop.

## Working constraints

- Use an isolated development database; this plan authorizes no deletion of an existing database or uploaded documents.
- Preserve the existing uncommitted design/probe work and unrelated user changes.
- Keep implementation/test changes small, show the actual diff, and follow the owner's explanation/approval checkpoint before proceeding or committing.
- This update changes documentation only. No application code, schema, dependency, or test changes have been made.
