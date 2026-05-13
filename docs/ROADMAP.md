# Roadmap

Date: 2026-05-13

## Current source and validation status

Current source checkpoint:

```text
Step 35 - pack trust policy preview with signature results
validation status: external Rust validation passed after bounded fixes
validated commit: 14b0a4f
validation date: 2026-05-13
```

External Rust validation passed end-to-end on the current Step 35 baseline:

```text
cargo fmt --check                                      PASS
cargo check                                           PASS
cargo test                                            PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo build --release                                 PASS
```

Validation environment reported by Codex:

```text
rustc 1.89.0 (29483883e 2025-08-04)
cargo 1.89.0 (c24e10642 2025-06-23)
```

## Progress summary

Use four scopes when describing progress:

| Scope | Current estimate | Notes |
|---|---:|---|
| Core safety spine through guarded remote PR creation | 100% | Core types, init, ledger, evidence, policy, manifest loading, providers, patch plan, approval records, approved patch apply, allowlisted test execution, local Git status, protected local branch creation, approved local commit scaffolding, local PR body generation, approval expiration enforcement, guarded remote PR creation, and Rust validation are complete for this baseline. |
| Local Developer MVP | 98-99% | The core local workflow is implemented, validated, and alpha-packaged. Remaining work is usability polish and fixes found by users. |
| Guarded remote PR MVP | 85-90% | GitHub PR creation exists, is disabled by default, dry-run by default, branch-constrained, duplicate-aware, and evidence/audit-backed. Remaining work is provider-specific polish and live validation. |
| Full Gadgets Framework roadmap | 56-60% | Developer workflow is alpha-packaged and validated through Step 35. Remote PR safety is hardened, redaction is centralized, pack trust/signing is designed, non-enforcing pack trust/root inspection is scaffolded, diagnostic evidence/audit is emitted, policy preview is signature-aware, signature metadata diagnostics are scaffolded, and Ed25519 signature verification diagnostics are implemented. Team workflows, Linux Server Admin packs, database/cloud/deployment packs, pack trust enforcement, signing tools, stronger secret handling, and UI/team integrations remain future work. |

## Phase 0 - Contract and skeleton

Status: complete for current baseline.

Completed:

- dual license selected: MIT OR Apache-2.0
- architecture baseline
- contract specs
- repository skeleton
- Rust workspace skeleton
- first vertical slice definition

## Phase 1 - Local observe runtime and provider-controlled handoffs

Status: complete and validated for the current local baseline.

Completed:

- core contract types and manifest validation
- local `.gadgets/` init
- append-only audit ledger with hash-chain verification
- evidence bundle writer and verifier
- deterministic built-in policy checks
- observe-only Filesystem Read provider
- `gadgets ask <request>` local repo inspection flow
- mock provider trait integration in the ask flow
- Coordinator stub using structured handoff objects
- `.gadgets/config.yaml` loading
- provider profile selection
- runtime mode passed into policy evaluation
- pack/Gadget manifest loading from installed `.gadgets/` state and built-in manifests
- `gadgets pack list`
- `gadgets pack show`
- `gadgets pack validate`
- OpenAI provider adapter
- Anthropic provider adapter

## Phase 2 - Local developer change workflow

Status: implemented and validated for the current baseline.

Completed:

- Patch Writer plan-only mode
- approval request/record scaffolding
- approved local patch apply
- allowlisted Test Runner
- approval expiration enforcement

Remaining polish:

- Documentation Gadget executable behavior
- Dependency Gadget plan-only behavior
- onboarding and error-message polish
- Developer MVP alpha packaging

## Phase 3 - Git and PR workflow

Status: implemented and validated for local workflow plus guarded GitHub PR creation.

Completed:

- local observe-only Git status
- protected local branch creation
- approved local commit scaffolding
- local PR body generation
- guarded remote GitHub PR creation behind explicit config

Completed hardening:

- remote PR dry-run mode
- allowed base branch configuration
- allowed head branch prefix configuration
- duplicate PR handling strategy
- API error evidence redaction polish

Remaining hardening:

- live provider-specific validation
- GitLab or other provider support, if later desired

## Implemented command surface

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
gadgets pack trust check [--project <path>] <pack>
gadgets pack trust roots [--project <path>]
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
gadgets pack trust signature [--project <path>] <pack>
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <YYYY-MM-DDTHH:MM:SSZ>]
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
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

## Current non-goals

Still not implemented and intentionally out of scope for the current baseline:

- arbitrary shell
- generic root-shell Gadget
- provider-side tool execution bypass
- Git push, fetch, pull, merge, or rebase
- Git checkout or switch
- remote branch creation
- GitLab PR/MR support
- Linux server administration behavior
- database behavior
- cloud behavior
- deployment behavior
- full secret scanner or DLP model
- pack trust enforcement and signing tools; Steps 26-29 define design, inspection, trust-root diagnostics, and future evidence/audit contracts only

## Phase 4 - Linux Server Admin Observe Pack

Status: deferred.

Planned:

- host inventory
- process and port inspector
- service observe
- firewall planner
- cleanup planner
- package/patch planner
- backup/restore verifier

## Phase 5 - Team workflows

Status: deferred.

Planned:

- shared approvals
- ticketing
- notifications
- team policy profiles
- shared ledger export

## Phase 6 - Controlled production/change packs

Status: deferred.

Planned:

- database planner/executor
- cloud readonly/change planner
- deployment planner/executor
- Linux Server Admin Change Pack

## Recommended next steps

### Step 23 - Developer MVP alpha packaging

Status: complete in this checkpoint.

Goal: make the validated Developer MVP easy to understand, configure, and use.

Completed:

- [x] Added `docs/DEVELOPER_MVP_ALPHA.md`.
- [x] Added a concise "what this can do today" section.
- [x] Added a concise "what this intentionally cannot do" section.
- [x] Added sample `.gadgets/config.yaml` snippets.
- [x] Added sample `test_commands` config.
- [x] Added safe disabled-by-default remote PR config example.
- [x] Added a complete command walkthrough.
- [x] Added troubleshooting notes.
- [x] Added safety model summary.
- [x] Added evidence/audit explanation.
- [x] Added known limitations.

### Step 24 - Remote PR safety hardening

Status: complete at checkpoint/code level in this checkpoint.

Goal: tighten the guarded remote PR path before broader use.

Completed:

- [x] Added remote PR dry-run mode.
- [x] Kept generated config dry-run by default.
- [x] Added config option for allowed base branches.
- [x] Added config option for allowed head branch prefixes.
- [x] Added duplicate PR handling strategy: `fail` or `reuse`.
- [x] Improved API error evidence redaction.
- [x] Avoided token reads in dry-run mode.
- [x] Preserved no Git push, fetch, pull, merge, rebase, checkout, or switch.

### Step 25 - Secret/output redaction hardening

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Goal: centralize and improve best-effort redaction for evidence outputs.

Completed:

- [x] Created shared redaction helper.
- [x] Applied helper consistently to Test Runner stdout/stderr, Git output, local PR body evidence, evidence summaries, and remote API responses.
- [x] Added tests for common secret-like patterns and UTF-8-safe truncation.
- [x] Documented limits clearly: this is not complete DLP or guaranteed secret scanning.

### Step 26 - Pack trust/signing design

Status: complete as a docs-first design checkpoint. Enforcement code is not implemented yet.

Goal: define how installed packs become trusted before third-party packs are supported.

Completed:

- [x] Defined pack identity.
- [x] Defined content manifest shape.
- [x] Defined detached signature record shape.
- [x] Defined local trust roots.
- [x] Defined verification failure behavior.
- [x] Defined Safe mode unsigned local behavior.
- [x] Defined Team/Production Mode requirements.
- [x] Added `specs/PACK_TRUST_SIGNING_SPEC.md`.

### Step 27 - Pack trust inspection scaffold

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Goal: add non-enforcing trust inspection before signature verification/enforcement.

Completed:

- [x] Added `gadgets pack trust check [--project <path>] <pack>`.
- [x] Reports built-in versus project-local pack source.
- [x] Reports `trusted_builtin`, `allowed_unsigned_local`, or `signed_metadata_unverified` diagnostic decisions.
- [x] Reports manifest SHA-256.
- [x] Inspects optional `pack.contents.yaml` and `pack.signature.yaml` metadata for project-local packs.
- [x] Reports basic manifest/content hash cross-check findings when signature metadata is present.
- [x] Reports whether `.gadgets/trust/trusted_publishers.yaml` is present.
- [x] Writes no mutations.
- [x] Does not enforce signed-pack requirements.
- [x] Does not add signing tools.
- [x] Does not add registry downloads or pack install/update commands.

### Step 28 - Trust root inspection scaffold

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Goal: add non-mutating trust-root diagnostics before trust-root mutation or signature enforcement.

Completed:

- [x] Added `gadgets pack trust roots [--project <path>]`.
- [x] Reports whether `.gadgets/trust/trusted_publishers.yaml` exists.
- [x] Reports parse status, version, publisher count, and publisher summaries.
- [x] Reports diagnostic findings for missing recommended fields.
- [x] Writes no mutations.
- [x] Does not verify signatures.
- [x] Does not enforce trust.
- [x] Does not add signing tools.
- [x] Does not add trust-root editing.
- [x] Does not add registry downloads or pack install/update commands.



### Step 29 - Pack trust evidence/audit design

Status: complete in this checkpoint.

Completed:

- [x] Defined future pack trust audit events.
- [x] Defined future signature verification audit events.
- [x] Defined future trust-root audit events.
- [x] Defined future pack trust evidence artifacts.
- [x] Defined future trust-root evidence artifacts.
- [x] Defined future pack-load denial evidence artifacts.
- [x] Documented private-key and secret-material exclusion from evidence/audit.

Superseded by Step 30 diagnostic evidence emission.


### Step 30 - Pack trust diagnostic evidence emission

Status: complete.

- [x] `gadgets pack trust check` writes diagnostic evidence.
- [x] `gadgets pack trust roots` writes diagnostic evidence.
- [x] Pack trust diagnostics append audit events.
- [x] Trust-root diagnostics append audit events.
- [x] Commands remain non-enforcing and non-mutating.
- [x] No cryptographic signature verification, signing tools, trust-root mutation, pack install/update, registry download, or Gadget execution behavior was added.


## Step 31 - Pack trust policy preview

- [x] Added `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>`.
- [x] Added non-enforcing Safe/Team/Production policy previews.
- [x] Added evidence and audit emission for policy previews.
- [x] Step 35 updates the preview to consume real signature diagnostics.
- [x] Preserved no enforcement, no signing tools, no trust-root mutation, no downloads, and no Gadget execution changes.

## Step 32 - Signature metadata verification scaffold

Status: complete at checkpoint level, external Rust validation deferred.

- [x] Added `gadgets pack trust signature [--project <path>] <pack>`.
- [x] Validates required signature metadata fields.
- [x] Validates strict UTC timestamp shape.
- [x] Validates pack identity and hash references.
- [x] Checks local trust-root publisher/key/algorithm references.
- [x] Writes diagnostic evidence and audit events.
- [x] Preserves non-enforcing behavior.
- [x] Ed25519 cryptographic verification diagnostics completed in Step 34.
- [ ] Pack trust enforcement remains deferred.

## Step 33 - Cryptographic verification design finalization

Status: complete as a documentation/specification checkpoint.

- [x] Locked Ed25519 as the first supported signature algorithm.
- [x] Locked SHA-256 as the hash algorithm.
- [x] Locked raw-byte hashing for `pack.yaml` and `pack.contents.yaml`.
- [x] Locked deterministic line-based `gadgets-pack-signature-v1` payload.
- [x] Locked content manifest verification rules.
- [x] Locked trust-root matching rules.
- [x] Locked denial mappings.
- [x] Locked evidence and audit expectations for real verification.
- [x] Preserved no cryptographic code and no enforcement in Step 33.

Step 34 is complete at checkpoint level.

## Step 34 - Ed25519 verification diagnostics

Status: complete at checkpoint level, external Rust validation deferred.

- [x] Added real Ed25519 signature verification to `gadgets pack trust signature`.
- [x] Recomputes raw-byte SHA-256 for `pack.yaml`.
- [x] Recomputes raw-byte SHA-256 for `pack.contents.yaml`.
- [x] Verifies every listed content file hash.
- [x] Requires `pack.yaml` to be listed in `pack.contents.yaml`.
- [x] Rejects `pack.signature.yaml` in signed content entries.
- [x] Rejects unsafe, duplicate, unsorted, missing, symlink, and non-file content entries.
- [x] Matches publisher, key id, algorithm, and allowed pack id against trust roots.
- [x] Checks signature and trust-root expiration metadata.
- [x] Verifies the deterministic `gadgets-pack-signature-v1` payload using Ed25519.
- [x] Writes signature verification evidence artifacts.
- [x] Preserves non-enforcing behavior.
- [x] No signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, or Gadget execution behavior changes were added.

Rust validation has not been rerun after Step 35.

## Step 35 - Pack trust policy preview with signature results

Status: complete at checkpoint level, external Rust validation deferred.

- [x] Updated `gadgets pack trust preview` to consume signature diagnostic results.
- [x] Safe Mode preview reports signature findings while still allowing local development packs.
- [x] Team Mode preview allows only valid trusted signatures diagnostically.
- [x] Production Mode preview allows only valid trusted signatures diagnostically.
- [x] Added signature policy inputs to preview evidence.
- [x] Preserved non-enforcing behavior.

Next recommended step: Step 36 - pack trust enforcement design and dry-run gate plan.
