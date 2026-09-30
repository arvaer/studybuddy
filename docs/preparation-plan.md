# Prepare the personal learning operator

Status: historical design snapshot, September 25, 2026. The [September 29 integration assessment](capsule-integration.md) supersedes the SDK readiness below; the subsequent [hardening-first plan](hardening-plan.md) controls implementation sequencing. It adds a tested standalone probe; the application adapter, persistence migrations, and frontend integration remain unimplemented.

Product direction: [personal operator contract](operator-design.md). Supporting designs: [learning experience](learning-design.md), [persistence](persistence-design.md).

## Work we can prepare now

| Artifact | Current status | What it should settle |
|---|---|---|
| Operator charter and invariants | Drafted in the operator contract | Shared guarantees, adaptable policies, learner control |
| Two reference journeys | Drafted below | Goal-dependent activities, evidence, and deliverables |
| Interface/block contract | Prose proposal in the operator contract | Versioning, inputs, action bindings, validation; executable schema remains to be designed |
| Default Anki and workspace exports | Format requirements drafted | Stable identities, provenance, media, history, portability |
| Capsule dependency map | Drafted below | Current public surface, Phase 3 prerequisites, later generation capabilities |
| Runtime adapter, renderer, and fixtures | Not implemented | Small code/test diffs after the corresponding review checkpoint |

Preparation should produce product contracts and independently reviewable examples. Avoid creating a substitute Capsule runtime while waiting for the real loop.

## Walkthrough A: RL thesis learner

**Goal:** reason about reward, return, and action value well enough to use them correctly in the routing project. Known background: NNs, transformers, NLP; interest in scratch PyTorch implementation. This preference suggests an available coding route, not a requirement for every lesson.

**Original exercise:** a robot chooses between A, which gives reward 2 and terminates, and B, which gives reward 0 followed by reward 5 and termination. Both outcomes are certain, discounting is absent, and there are no other costs. The returns are 2 and 5. Under the stated fixed continuation, B has the higher action value despite its lower immediate reward.

This is an authored design example, not a quotation from Sutton and Barto. A book-linked version needs verified passages from the uploaded edition. Do not invent page anchors.

**Initial view:** the problem statement, two action choices, an explanation field, and optional hint/source controls. Ask for a prediction before revealing the return timeline.

| Observed response | Proposed follow-up | What is recorded |
|---|---|---|
| Chooses A because its immediate reward is higher | Show the two-step timeline, then ask the learner to compute both totals | Attempt, misconception hypothesis, selected follow-up, assistance |
| Chooses B and explains the future payoff | Present a different problem where waiting is worse | Explanation evidence and a separate application attempt |
| Chooses B but says waiting is always better | Contrast two cases with different later rewards | Correct selection with incomplete reasoning; no blanket mastery update |
| Requests help or is unsure | Offer a graduated hint and a worked example, then a fresh question | Assisted practice, followed separately by any unaided result |
| Free-text assessment is uncertain | Ask a focused clarification or show the assessment for correction | Pending/uncertain assessment rather than an invented correct/incorrect label |

An optional artifact is a tiny return-calculation exercise to implement locally. Code execution is not required for this first activity. The operator can produce a note connecting the example to the thesis, clearly separating the learner's explanation, verified source claims, and proposed research hypotheses.

**Follow-up evidence:** an unaided problem with changed context and payoff structure after a defined delay. Choose its timing before the pilot; measure it separately from immediate repaired answers.

**Export:** the learning note in Markdown, the original task and attempts, bibliography entries for consulted sources, and any code/plots with versions and execution status.

## Walkthrough B: organic chemistry exam learner

**Goal:** prepare for a specific exam. Required inputs are the syllabus or learning objectives, exam date, available study time, and course-approved material. Those inputs are not present yet, so this remains a reference scenario rather than a generated chemistry lesson.

The operator proposes a bounded unit from the course material and creates a reviewable set of recall cards. Each card has a stable identity, a clear expected response, a source reference, and any required structure/reaction media. Include application questions where the actual exam requires them. Chemistry content and diagrams need subject-appropriate validation before publication.

**Initial view:** today's review, remaining syllabus coverage, a short practice activity, and Export to Anki. Source lookup and explanations stay available without forcing the researcher-oriented workspace onto this learner.

**Adaptation:** repeated confusion between two reactions can trigger a comparison activity. Repeated successful recall can change review timing. Performance on representative exam problems informs the balance of recall and application; the count of completed cards does not determine readiness.

**Export:** packaged Anki deck plus TSV, source references, and required media. Re-export should update stable notes under a tested policy; it should not silently erase local edits or reset reviews. Until history import is implemented, Anki-managed cards have unknown current review status inside StudyBuddy.

**Follow-up evidence:** delayed recall and unseen course-relevant questions. An intended grade is a goal, not a promised outcome.

## Capsule integration dependencies

Inspected local Capsule checkout: `3bc76549331721e37a981cede1cff3e81e95a114`. The checkout is under active development; verify the public SDK and its contracts again before implementation.

Contract sources:

- [SDK, “The box” and “The record”](/Users/mikeyalmeida/Documents/capsule-corp/paos/capsule-corp/.active/sdk-surface-v1.md): storage, session lifecycle, provider crossings, replay, single-owner recording.
- [Authority algebra, “Emergent capabilities: programs, not authority” and “The substrate: CAS nodes”](/Users/mikeyalmeida/Documents/capsule-corp/paos/capsule-corp/.active/authority-algebra-v0.md): composition under existing authority and the node substrate.
- [Build plan, “Plan 3” and “v1, sketched”](/Users/mikeyalmeida/Documents/capsule-corp/paos/capsule-corp/.active/CURRENT.md): implementation sequencing, not an additional contract.

Current public building blocks include `Storage`, `Session::open/reopen`, instantiate/run, provider replies, and feed. The reviewed SDK still lists parks, the router, and the high-level loop as planned. Code sketches in the contract are not permission to depend on unshipped methods.

| Milestone | Why StudyBuddy needs it | Integration acceptance example |
|---|---|---|
| P3-01–05: parks and resolution | Wait for a learner or provider, then resume the right activity | Reopen while waiting; an answer binds to the intended pending interaction and cannot authorize unrelated work |
| P3-06–07: routing and recorded decisions | Choose among permitted teaching activities | The choice and its context survive replay without asking the model again |
| P3-08–10b: loop, input, budget, instances | Operate a continuing learning workspace | Duplicate input has one logical effect; concurrent UI requests are serialized by the host |
| P3-11–12: uncertain delivery and crash matrix | Recover without inventing outcomes or repeating external actions blindly | Interrupt after a provider commits but before its reply is recorded; reconcile through the supported protocol |
| P3-14, 14b, 14c: public surface and crossings | Depend on supported APIs and declared shapes | The lesson runs through public interfaces and rejects unsupported inputs/outputs |
| P3-15: perception | Optional direct PDF perception workflow | A recorded document-reading step reopens without rerendering or calling a model |
| v1 synthesis, promotion, and confinement | Develop and reuse new executable building blocks | A generated tool is admitted, confined, checked at its return, and pinned by version |

Phase 3 is the target for the first integrated operator using supplied capabilities. Generating content and composing existing components can precede arbitrary generated-code execution. A custom browser widget needs a separate host rendering/isolation design; executing synthesized build or analysis code also needs the confinement required by SDK S5/S9. Phase 3 completion alone does not establish those paths.

## Storage and host responsibilities to resolve

Keep a persistent learner/workspace identity separate from the lifecycle of a Capsule session. Start with one owned session for the active workspace and serialize its mutations. Core's algebra lists unbounded record growth and linear reopen cost as an open compaction issue; do not assume an indefinitely growing lifetime session is already solved.

Implement the public storage port for immutable Capsule nodes and compare-and-swap refs. Preserve the SDK's canonical node encoding; raw PDFs and HTML are application artifacts rather than new core primitive node types. The current `Storage` interface is synchronous, while StudyBuddy uses asynchronous SQLx: the adapter's execution/connection strategy needs an explicit design before implementation.

Application providers persist source artifacts, attempts, assessments, and published views, returning stable receipts that tie their domain records to the Capsule effect identity. The provider commit and the core reply append are separate crash boundaries. The host must use idempotent publication and the supported uncertain-delivery protocol; database transactions alone do not make external effects exactly once.

The runtime records how the operator acted. PostgreSQL owns the accepted application records and serves read models; each read model identifies its input versions. The host serves the selected interface artifact and checks every resulting action against the learner, workspace, revision, and allowed operation.

## First implementation checkpoints after design review

1. A single trusted, fixture-driven lesson view with a typed interaction contract. Label fixture behavior as such; it is not a working personal operator.
2. A durable domain command and receipt for one learner attempt, with retry and revision behavior tested.
3. The Capsule storage/provider adapter against a pinned supported SDK, with reopen and interruption verification.
4. One integrated personal operator that selects a different follow-up based on a recorded attempt.
5. The second reference journey and default Anki export, demonstrating that the product supports a different goal without a separate core architecture.
6. New block generation and promotion as the required runtime capabilities become available.

Each implementation or test diff is a separate comprehension/review checkpoint under the owner's collaboration instructions. None of these implementation steps has been started. The initial experimental question is whether operator adaptation improves the usefulness and learning outcomes of a good starting harness enough to justify its complexity.
