# Gadgets Framework - Updated Plan and Progress After Step 17

Date: 2026-05-12

## Progress summary

Best single answer: about one-third of the full Gadgets Framework roadmap is complete.

More useful breakdown:

- Core safety spine through allowlisted test execution: 100% at checkpoint/code level.
- Local Developer MVP: 82-85% complete.
- Full multi-pack roadmap: 30-35% complete.

The core local developer flow now includes repo observation, patch proposal, approval records, approved local patch application, and allowlisted test execution. The broader roadmap still includes Git and PR workflow, Team workflows, Linux Server Admin packs, database/cloud/deployment packs, production-mode controls, pack trust/signing, stronger secret handling, and UI/team integrations.

## Completed milestones

### Planning and architecture

- [x] Concept and architecture reference
- [x] Project plan
- [x] Linux Server Admin pack family plan
- [x] Rust core runtime direction
- [x] Phase 0 technical contracts
- [x] Decision record
- [x] Initial implementation plan
- [x] Repo skeleton
- [x] Markdown documentation rule

### Core implementation

- [x] Rust workspace skeleton
- [x] Core Gadget/Pack manifest types
- [x] Capability and permission model
- [x] Zone/boundary structs
- [x] Handoff/action/policy/evidence/audit structs
- [x] Manifest validation
- [x] `.gadgets/` init
- [x] Safe Mode default config
- [x] Append-only audit ledger
- [x] Audit hash chaining and verification
- [x] Evidence bundle writer
- [x] Evidence artifact hashing and verification
- [x] Deterministic policy engine
- [x] Filesystem Read observe-only provider
- [x] Mock provider and Coordinator stub
- [x] Config loading and provider profile selection
- [x] Pack/Gadget manifest loader
- [x] Pack validation
- [x] OpenAI provider adapter
- [x] Anthropic provider adapter
- [x] Patch Writer plan-only mode
- [x] Approval request/record scaffolding
- [x] Approved local patch application
- [x] Allowlisted Test Runner

## Current supported local flows

- [x] `gadgets init`
- [x] `gadgets ask` for repo inspection
- [x] `gadgets ask` for plan-only patch proposal
- [x] `gadgets approval request-patch`
- [x] `gadgets approval approve`
- [x] `gadgets approval verify`
- [x] `gadgets patch apply`
- [x] `gadgets test run`
- [x] `gadgets evidence verify`
- [x] `gadgets ledger verify`

## Current command surface

```bash
gadgets init [path]
gadgets ask [--project <path>] <request>
gadgets ledger show [project-root-or-ledger-path]
gadgets ledger verify [project-root-or-ledger-path]
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
gadgets pack validate [--project <path>] [--strict] [pack]
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]
gadgets approval approve [--project <path>] <approval-request-id> <approver>
gadgets approval show [--project <path>] <approval-request-id>
gadgets approval verify [--project <path>] <approval-request-id>
gadgets approval id-for-run <run-id>
gadgets patch apply [--project <path>] <approval-request-id>
gadgets test run [--project <path>] <test-command-name>
```

## Step 17 - Allowlisted Test Runner

Status: implemented at checkpoint/code level.

Acceptance checklist:

- [x] A named allowlisted test command can be routed through the explicit CLI path.
- [x] Unknown command names are rejected.
- [x] Commands are loaded from `.gadgets/config.yaml`.
- [x] Arbitrary command strings are not accepted from prompts, model output, or CLI free text.
- [x] Configured commands are launched directly with `std::process::Command`, not `sh -c`.
- [x] Shell metacharacters and unsupported quoting are rejected in configured commands.
- [x] `working_dir` is constrained to the project boundary.
- [x] Parent traversal is rejected.
- [x] Runtime policy checks `test.run` before execution.
- [x] stdout is captured.
- [x] stderr is captured.
- [x] exit status is captured.
- [x] duration is captured.
- [x] pass/fail status is recorded.
- [x] test failure is recorded without hiding output.
- [x] evidence bundle is written.
- [x] audit ledger events are appended.
- [x] test evidence is separate from patch apply evidence.
- [x] no patch application happens inside the Test Runner.
- [x] no Git/PR behavior is added.
- [x] no Linux admin behavior is added.
- [x] docs and specs are updated.

Validation note: this checklist reflects implemented source behavior at checkpoint level. Cargo/Rust validation still needs to be run externally.

## Remaining local Developer MVP work

### Step 18 - Git status/branch/commit scaffolding

- [ ] Add Git provider scaffolding
- [ ] Add observe-only `git.status`
- [ ] Add branch creation behind explicit config and policy
- [ ] Add commit of approved files only
- [ ] Refuse protected branch mutation
- [ ] Attach evidence references
- [ ] No remote push by default

### Step 19 - PR body and optional PR creation

- [ ] Generate PR title/body
- [ ] Include patch summary
- [ ] Include test result summary
- [ ] Include risk notes
- [ ] Include evidence references
- [ ] Gate remote PR creation behind config

## Future roadmap

### Phase 2 - Developer Pack polish

- [ ] Documentation Gadget executable behavior
- [ ] Code Review Gadget
- [ ] Dependency Gadget plan-only mode
- [ ] Better onboarding flow
- [ ] Better examples
- [ ] Better error messages
- [ ] Guided configuration
- [ ] Provider hardening

### Phase 3 - Team workflows

- [ ] Ticketing Gadget
- [ ] Notification Gadget
- [ ] Shared approval workflow
- [ ] Shared audit export
- [ ] Team policy profiles
- [ ] GitHub/GitLab integration hardening
- [ ] Evidence reports

### Phase 4 - Linux Server Admin Observe Pack

- [ ] Linux Host Inventory
- [ ] Process and Port Inspector
- [ ] Service Manager Observe
- [ ] Firewall Planner
- [ ] Filesystem Cleanup Planner
- [ ] Package and Patch Planner
- [ ] Backup and Restore Verifier

### Phase 5 - Linux Server Admin Change Pack

- [ ] Service Manager
- [ ] Package and Patch Executor
- [ ] Firewall Executor
- [ ] Filesystem Cleanup Executor
- [ ] Config Editor
- [ ] Container Runtime
- [ ] CI Runner Helper
- [ ] Maintenance Window
- [ ] Reboot Coordinator

### Phase 6 - Database, cloud, and deployment packs

- [ ] Database Readonly
- [ ] Migration Planner
- [ ] Migration Executor
- [ ] Cloud Readonly
- [ ] Infrastructure Planner
- [ ] Infrastructure Change
- [ ] Deployment Planner
- [ ] Deployment Executor
- [ ] Observability/verification workflows

## Known gaps to carry forward

- [x] Historical item superseded: Rust validation passed at Step 22 commit c5fbd78.
- [ ] Test Runner does not provide OS-level sandboxing around launched commands.
- [ ] Test output redaction is first-pass only and not a complete secret scanner.
- [ ] Approval expiry format/enforcement not fully locked.
- [ ] Patch apply remains intentionally narrow.
- [ ] Evidence failure after writes needs future hardening.
- [ ] Git/PR not implemented.
- [ ] Linux admin packs are placeholders.
- [ ] Secret handling needs stronger future scanner/redaction/handle model.
- [ ] Pack signing/trust model not implemented.

## External validation commands

Run these when a Rust toolchain is available:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Proceed with Step 18 - Git status/branch/commit scaffolding.

Keep Step 18 narrow: local Git status first, explicit policy checks, evidence and audit for meaningful work, no remote push by default, and no PR creation until a later gated step.
