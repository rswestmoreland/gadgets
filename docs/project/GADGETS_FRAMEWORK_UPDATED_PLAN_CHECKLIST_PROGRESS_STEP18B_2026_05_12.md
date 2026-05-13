# Gadgets Framework - Updated Plan and Progress After Step 18b

Date: 2026-05-12

## Progress summary

- Core safety spine through protected local Git branch creation: 100% at checkpoint/code level.
- Local Developer MVP: 87-89% complete.
- Full multi-pack roadmap: 32-37% complete.

## Completed milestones

- [x] Step 2: Core types and manifest parsing
- [x] Step 3: `gadgets init` and local `.gadgets` project state
- [x] Step 4: append-only audit ledger with hash-chain verification
- [x] Step 5: evidence bundle writer with artifact hashes
- [x] Step 6: deterministic policy engine v0.1
- [x] Step 7: observe-only Filesystem Read Gadget wired through policy, evidence, and audit
- [x] Step 8: deterministic mock provider and richer Coordinator stub
- [x] Step 9: config loading and provider profile selection
- [x] Step 10: installed pack and Gadget manifest loading
- [x] Step 11: `gadgets pack validate`
- [x] Step 12: OpenAI provider adapter
- [x] Step 13: Anthropic provider adapter
- [x] Step 14: Patch Writer plan-only mode
- [x] Step 15: approval request/record scaffolding
- [x] Step 16: approved local patch application
- [x] Step 17: allowlisted Test Runner
- [x] Step 18a: local observe-only Git status
- [x] Step 18b: protected local Git branch creation

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
```

## Step 18b implemented boundary

- [x] Fixed local Git branch command only
- [x] No arbitrary Git arguments
- [x] No shell
- [x] Branch name validation before execution
- [x] Protected branch config under `git.protected_branches`
- [x] Exact protected branch names rejected
- [x] Protected branch prefixes ending in `/` rejected
- [x] No checkout or branch switching
- [x] No staging
- [x] No commit creation
- [x] No push, pull, fetch, merge, rebase, or remote operation
- [x] No PR creation
- [x] Evidence bundle written
- [x] Audit events appended
- [x] Secret-like Git output lines redacted before evidence write

## Remaining local Developer MVP work

### Step 18c - Approved local commit scaffolding

- [ ] Bind commit to approved patch evidence
- [ ] Bind commit to test-run evidence when configured
- [ ] Stage only approved changed files
- [ ] Reject unapproved files
- [ ] Reject commits on protected branches
- [ ] Require clean or expected working tree shape
- [ ] No remote push
- [ ] Evidence bundle for commit

### Step 19 - PR body and optional PR creation

- [ ] Generate PR title/body
- [ ] Include patch summary
- [ ] Include test result summary
- [ ] Include Git status/branch summary
- [ ] Include risk notes and evidence references
- [ ] Gate remote PR creation behind explicit config

## Known gaps to carry forward

- [x] Historical item superseded: Rust validation passed at Step 22 commit c5fbd78
- [ ] Approval expiry format/enforcement not fully locked
- [ ] Patch apply remains intentionally narrow
- [ ] Evidence failure after writes needs future hardening
- [ ] Git commit creation not implemented
- [ ] Git remote behavior and PR creation not implemented
- [ ] Linux admin packs are placeholders
- [ ] Secret handling needs stronger future scanner/redaction/handle model
- [ ] Pack signing/trust model not implemented

## External validation commands

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
