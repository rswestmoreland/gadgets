# Gadgets Framework - Updated Plan and Progress After Step 20

Date: 2026-05-13

## Progress summary

Best single answer: about 35-40 percent of the full Gadgets Framework roadmap is complete.

More useful breakdown:

- Core safety spine through local PR body generation and Step 20 hardening: 100% at checkpoint/code level.
- Local Developer MVP: historical estimate. Step 22 later completed external Rust validation at commit c5fbd78; remaining work is alpha packaging and polish.
- Full multi-pack roadmap: 35-40% complete.

## Completed local Developer milestones

- [x] Core manifest/type model
- [x] `.gadgets/` init
- [x] append-only audit ledger
- [x] evidence bundles
- [x] deterministic policy engine
- [x] Filesystem Read observe flow
- [x] mock provider and Coordinator stub
- [x] config loading and provider profile selection
- [x] pack/Gadget manifest loading
- [x] pack validation
- [x] OpenAI provider adapter
- [x] Anthropic provider adapter
- [x] Patch Writer plan-only mode
- [x] approval request/record scaffolding
- [x] approved local patch application
- [x] allowlisted Test Runner
- [x] local Git status
- [x] protected local branch creation
- [x] approved local commit scaffolding
- [x] local PR body generation
- [x] approval expiration format and enforcement
- [x] Local Developer MVP walkthrough documentation

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
gadgets git status [--project <path>]
gadgets git branch create [--project <path>] <branch-name>
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

## Step 20 completed

- [x] Approval expiration format locked to strict UTC RFC3339 without fractional seconds.
- [x] Invalid expiration values rejected at approval request creation.
- [x] Expired approval requests rejected before approval recording.
- [x] Expired approvals rejected by approval verification.
- [x] Existing patch apply, approved commit, and PR body paths continue to rely on approval verification.
- [x] CLI help updated for `--expires-at`.
- [x] Local Developer MVP walkthrough added.
- [x] README, docs, specs, roadmap, and example config reconciled.
- [x] No remote behavior added.

## Known gaps

- [ ] External Rust validation still required.
- [ ] Evidence failure after mutation needs future hardening.
- [ ] Redaction is basic and not a full secret scanning system.
- [ ] Remote PR creation is not implemented.
- [ ] Git push/pull/fetch/merge/rebase are not implemented.
- [ ] Linux admin packs are placeholders.
- [ ] Database/cloud/deployment packs are not implemented.
- [ ] Pack signing/trust model is not implemented.
- [ ] Documentation Gadget executable behavior is not implemented.
- [ ] Dependency Gadget behavior is not implemented.

## Historical validation request superseded by Step 22

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Historical note: Step 22 later completed the external Rust validation flow and bounded fixes at commit c5fbd78.

After validation is green, choose between:

1. Developer MVP release packaging and polish, or
2. remote PR creation planning behind explicit configuration and additional approval gates.
