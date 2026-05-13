# Gadgets Framework - Session Wrap-Up Review

Date: 2026-05-12

## Scope reviewed

Reviewed the Step 16 checkpoint bundle after approved local patch application was implemented.

Reviewed areas:

- root README
- architecture and roadmap docs
- implementation plan
- specs that mention patch planning/application
- CLI command surface
- approval flow
- patch apply provider
- file manifest
- ASCII/path-length/static consistency

## Current progress estimate

Use three percentages because the plan has multiple scopes:

| Scope | Estimate | Notes |
|---|---:|---|
| Core safety spine through approved local patch application | 100% | Core types, init, ledger, evidence, policy, manifest loading, providers, patch plan, approval records, and approved patch apply are implemented at code/checkpoint level. |
| Local Developer MVP | 75-80% | Remaining major items are allowlisted Test Runner, Git status/branch/commit prep, and PR body/optional PR integration. |
| Full Gadgets Framework roadmap | 30-35% | Developer Pack core is well underway, but Team workflows, Linux Server Admin packs, database/cloud/deployment packs, pack trust/signing, and production-mode hardening remain future work. |

Best single answer: **about one-third of the full roadmap is complete**, and **roughly three-quarters of the local Developer MVP is complete**.

## Drift fixed during wrap-up

### README status drift

Fixed stale README text that still described patch application as deferred. README now reflects that Step 16 implements approved local patch application through `gadgets patch apply`.

### Roadmap drift

Fixed stale roadmap text that said patch application remained deferred after Step 15 and that Anthropic was still pending. Roadmap now reflects:

- mock, OpenAI, and Anthropic provider adapters are implemented
- Patch Writer plan-only mode is implemented
- approval records are implemented
- approved local patch application is implemented
- Step 17 is the next milestone

### Architecture drift

Fixed architecture wording that still described only the observe-only Filesystem Read runtime slice. Architecture now describes the current local Developer Pack slices:

- Filesystem Read observe-only repo inspection
- Patch Writer plan-only evidence
- approved local patch application through `gadgets patch apply`

### Pack model drift

Fixed Pack Model wording that still said patch application was deferred. It now states that Patch Writer supports plan-only patch evidence and approved local patch application.

### Evidence spec drift

Fixed wording that described evidence linkage as future-only. Evidence linkage now reflects current Filesystem Read, Patch Writer plan, approval, and patch apply flows.

## Code safety issue fixed during wrap-up

### Potential partial multi-file patch application

Issue found:

The Step 16 patch apply provider checked policy for every target first, but then applied files one by one. If an early file was written successfully and a later file failed hunk matching, the patch could partially apply.

Fix made:

`crates/gadgets-tools/src/patch_apply.rs` now prepares all file changes before any write:

1. verify approval request and approval record
2. verify scope hash and patch SHA-256
3. parse the full supported unified diff
4. run policy checks for every target path
5. compute every target file's before/after content and hashes
6. only then write files

This reduces the chance of partial writes from a later hunk mismatch.

## Remaining known gaps

These are known and should be carried into the next session.

### Historical validation note superseded by Step 22

Cargo/Rust is not available in this sandbox. The project later completed external validation at Step 22 commit c5fbd78. Original requested commands were:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

### Approval expiration is represented but not fully enforced

Approval records can include `expires_at`, but the format and enforcement semantics should be locked before production use. Recommended Step 17 or Step 18 follow-up:

- define a strict timestamp format
- enforce expiry in `verify_approval()` or patch/test execution entrypoints
- add tests for expired approvals

### Patch apply is intentionally narrow

Current patch application supports a constrained unified-diff subset. It intentionally does not support:

- file deletion
- binary patches
- rename/copy metadata
- mode changes
- arbitrary shell patch application
- rollback execution

This is acceptable for Step 16 and should stay narrow until the safety model is validated externally.

### Evidence generation after writes can still fail

Patch apply writes files before final evidence bundle creation. If evidence writing fails after successful file writes, the runtime may have a mutation without complete evidence. This is a future hardening item.

Possible later mitigation:

- pre-create an apply-attempt evidence directory before writing
- append a pre-write audit event
- write final evidence and verification after mutation
- define failure handling for evidence-write failures

### Test Runner not implemented

No tests are run after patch application yet. Step 17 should add an allowlisted Test Runner.

### Git/PR not implemented

No Git status, branch, commit, push, or PR creation behavior exists yet.

### Linux admin packs are placeholders

Linux Server Admin Observe and Change packs exist as pack-level placeholders only. No Linux admin behavior exists.

## Current implemented command surface

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
```

## Immediate next step

Proceed with **Step 17 - Allowlisted Test Runner**.

Do not add arbitrary shell. Test commands must be explicitly configured in `.gadgets/config.yaml` and run through a narrow provider that records stdout/stderr, exit status, evidence, and audit events.
