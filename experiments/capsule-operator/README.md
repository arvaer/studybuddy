# StudyBuddy / Capsule compatibility probe

First review slice for the [operator integration](../../docs/capsule-integration.md). This standalone crate does not change StudyBuddy's backend workspace or frontend. It exercises real Capsule execution with scripted model replies and in-memory publication receipts. There is no model API, database, network provider, grading, or rendered UI.

Core is pinned to `7ad7c4bfe36b45667a77f222c2a1772ad0a0ae60`. Use Rust 1.97.0. From the StudyBuddy repository:

```sh
cargo +1.97.0 fmt --manifest-path experiments/capsule-operator/Cargo.toml --check
cargo +1.97.0 clippy --locked --manifest-path experiments/capsule-operator/Cargo.toml --all-targets -- -D warnings
cargo +1.97.0 test --locked --manifest-path experiments/capsule-operator/Cargo.toml
```

Dependency fetching requires access to the Capsule repository. A local development override is possible with Cargo's `patch` configuration, but it must use the pinned revision to reproduce this result; overrides can change the lockfile. Initial compatibility tests used an isolated `git archive` of that revision. Final formatting, Clippy (`-D warnings`), and all three tests also passed against the Git dependency and lockfile included here.

The three tests cover lesson publication and learner wait across reopen, duplicate/conflicting starts, refusal before another workspace is touched, and receipt reconciliation after uncertain publication. The model's follow-up is scripted; this establishes execution behavior, not the quality of adaptation. `MemoryStorage` replay is not proof of database or power-loss durability.

The capsule offers three verbs: ask, propose, and finish. Ask presents the activity before waiting for the learner. The environment and capsule explicitly scope domain effects to `workspaces/rl/*`; production scope compilation must use trusted workspace IDs. The proposed follow-up remains unassessed.

Receipts use a tagged list because [SDK issue #284](https://github.com/Prominent-Systems/capsule-corp/issues/284) prevented observing object/float provider results through `resolve` at the first pin; it is fixed at the current pin, and the probe keeps the tagged list so the pin bump changes no behaviour (the application providers in #19 use objects). The issue contains the standalone reproducer. This probe does not modify Core or introduce a second agent runtime.
