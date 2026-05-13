# Gadgets Framework - Updated Plan and Progress After Step 19

Date: 2026-05-12

## Progress summary

- Core safety spine through local PR body generation: complete at checkpoint/code level.
- Local Developer MVP: about 90-92% complete.
- Full multi-pack roadmap: about 34-38% complete.

The local Developer Pack now covers repo inspection, plan-only patch proposal, scoped approval, approved patch apply, allowlisted tests, local Git status, protected local branch creation, approved local commit scaffolding, and local PR body generation.

## Completed local Developer milestones

- [x] Core manifest and policy types
- [x] `.gadgets/` init
- [x] append-only audit ledger
- [x] evidence bundle writer
- [x] deterministic policy engine
- [x] Filesystem Read observe-only Gadget
- [x] mock/OpenAI/Anthropic provider adapters
- [x] installed pack and Gadget manifest loading
- [x] pack validation
- [x] Patch Writer plan-only mode
- [x] approval request and approval record scaffolding
- [x] approved local patch application
- [x] allowlisted Test Runner
- [x] local Git status
- [x] protected local branch creation
- [x] approved local Git commit scaffolding
- [x] local PR body generation

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

## Remaining local Developer MVP work

- [ ] external Rust validation and fixes
- [ ] approval expiry format and enforcement
- [ ] better guided configuration
- [ ] better onboarding examples
- [ ] optional remote PR creation planning behind explicit config
- [ ] stronger secret handling and redaction model
- [ ] evidence failure hardening after writes

## Future roadmap

- Team workflows
- Linux Server Admin Observe Pack
- Linux Server Admin Change Pack
- database/cloud/deployment packs
- pack signing/trust model
- production-mode controls
- UI/team integrations

## Recommended next step

Run external Rust validation. If validation is clean or after bounded fixes, proceed to remote PR creation design only. Remote PR creation must stay disabled by default and require explicit configuration.
