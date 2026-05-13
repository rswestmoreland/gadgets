# Gadgets Framework - Updated Plan and Progress After Step 18c

Date: 2026-05-12

## Progress summary

Best single answer: about one-third of the full Gadgets Framework roadmap is complete.

More useful breakdown:

- Core safety spine through approved local Git commit scaffolding: 100% at checkpoint/code level.
- Local Developer MVP: 90-92% complete.
- Full multi-pack roadmap: 35-40% complete.

The Local Developer MVP moved forward because Step 18 now includes local Git status, protected local branch creation, and approved local commit scaffolding. Remaining local Developer MVP work is mainly PR body generation, optional PR creation behind explicit configuration, and external Rust validation/hardening.

## Completed milestones

- [x] Step 2: Core types and manifest parsing
- [x] Step 3: gadgets init and local .gadgets project state
- [x] Step 4: append-only audit ledger with hash-chain verification
- [x] Step 5: evidence bundle writer with artifact hashes
- [x] Step 6: deterministic policy engine v0.1
- [x] Step 7: observe-only Filesystem Read Gadget wired through policy, evidence, and audit
- [x] Step 8: deterministic mock provider and richer Coordinator stub
- [x] Step 9: config loading and provider profile selection
- [x] Step 10: installed pack and Gadget manifest loading
- [x] Step 11: gadgets pack validate
- [x] Step 12: OpenAI provider adapter
- [x] Step 13: Anthropic provider adapter
- [x] Step 14: Patch Writer plan-only mode
- [x] Step 15: approval record scaffolding for local writes
- [x] Step 16: approved local patch application
- [x] Step 17: allowlisted Test Runner
- [x] Step 18a: local Git status
- [x] Step 18b: protected local branch creation
- [x] Step 18c: approved local commit scaffolding

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
```

## Step 18c acceptance checklist

- [x] Approved local commit command added.
- [x] Commit command verifies approval request and approval record.
- [x] Commit command verifies exact patch hash and scope hash.
- [x] Commit command extracts approved files from `proposed.patch`.
- [x] Detached HEAD rejected.
- [x] Protected current branches rejected.
- [x] Preexisting staged changes rejected.
- [x] Only approved patch files are staged.
- [x] Staged file set is verified before commit.
- [x] Commit is local only.
- [x] No checkout/switch behavior added.
- [x] No push/pull/fetch/merge/rebase behavior added.
- [x] No PR behavior added.
- [x] No arbitrary shell added.
- [x] Evidence bundle is written.
- [x] Audit ledger events are appended.
- [x] Docs/specs/README/roadmap updated.
- [ ] External Rust validation still required.

## Still not implemented

- Git PR body generation
- Optional PR creation
- Git push/pull/fetch/merge/rebase
- Linux admin behavior
- database/cloud/deployment behavior
- arbitrary shell
- provider-side tool execution
- OS-level sandboxing for test processes
- full secret scanner/redaction model
- approval expiry enforcement

## Recommended next step

Proceed to Step 19: PR body generation first. Optional PR creation should remain disabled by default and require explicit configuration.
