# Roadmap

Date: 2026-05-13

## Current source and validation status

Current source checkpoint:

```text
Step 43 - Provider and model inventory design
source baseline: Step 42 AI RMF governance profile checkpoint based on Step 35 externally validated commit 14b0a4f
Step 43 change type: docs/spec/config-example update only
validation status: ready for external Rust validation in Codex
```

External Rust validation passed end-to-end on the Step 35 source baseline. Steps 37 through 41 require a new external validation run now. Steps 42 and 43 are docs/spec only. Required commands:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
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
| Local Developer MVP | 99% | The core local workflow is implemented, validated through Step 35, alpha-packaged, and now has dry-run trust gate reporting and summary review. Step 42 does not change runtime behavior. Remaining work is usability polish, external validation of Steps 37 through 41, and fixes found by users. |
| Guarded remote PR MVP | 85-90% | GitHub PR creation exists, is disabled by default, dry-run by default, branch-constrained, duplicate-aware, and evidence/audit-backed. Remaining work is provider-specific polish and live validation. |
| Full Gadgets Framework roadmap | 66-69% | Developer workflow is alpha-packaged and validated through Step 35. Step 36 completes the docs-first enforcement design, Step 37 implements the narrow dry-run-only pack-load trust gate, Step 38 adds operator preview reporting, Step 39 adds gate-history reporting, Step 40 adds gate-status reporting, Step 41 adds gate-summary reporting, Step 42 adds the first AI RMF governance alignment design, and Step 43 adds provider/model inventory design. Remote PR safety is hardened, redaction is centralized, pack trust/signing is designed, diagnostics and Ed25519 verification exist, and runtime dry-run trust warnings/denials now have evidence/audit and operator summaries. Team workflows, Linux Server Admin packs, database/cloud/deployment packs, hard-deny enforcement, signing tools, stronger data exposure controls, AI risk reporting, and UI/team integrations remain future work. |

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
gadgets pack trust gate-status [--project <path>]
gadgets pack trust gate-summary [--project <path>]
gadgets pack trust gate-preview [--project <path>] [--operation <operation>] <pack>
gadgets pack trust gate-history [--project <path>] [--limit <n>]
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
- hard-deny pack-load enforcement and signing tools; Steps 26-41 define design, diagnostics, signature verification, policy preview, future evidence/audit contracts, dry-run gate behavior, and read-only gate reporting
- runtime AI risk inventory, AI RMF reporting, AI incident workflows, and data exposure enforcement; Step 42 defines the first docs/spec alignment model and Step 43 defines provider/model inventory

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

External validation is now the immediate next action. Steps 42 and 43 are docs/spec only, but Steps 37 through 41 changed Rust source and must be validated before additional runtime features. Do not add hard-deny behavior until dry-run evidence is reviewed and explicitly approved.

### Step 23 
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

Status: complete and included in validated commit `14b0a4f`.

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

Status: complete and included in validated commit `14b0a4f`.

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

Status: complete and included in validated commit `14b0a4f`.

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

Status: complete and included in validated commit `14b0a4f`.

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

Step 34 is complete and included in validated commit `14b0a4f`.

## Step 34 - Ed25519 verification diagnostics

Status: complete and included in validated commit `14b0a4f`.

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

## Step 35 - Pack trust policy preview with signature results

Status: complete and included in validated commit `14b0a4f`.

- [x] Updated `gadgets pack trust preview` to consume signature diagnostic results.
- [x] Safe Mode preview reports signature findings while still allowing local development packs.
- [x] Team Mode preview allows only valid trusted signatures diagnostically.
- [x] Production Mode preview allows only valid trusted signatures diagnostically.
- [x] Added signature policy inputs to preview evidence.
- [x] Preserved non-enforcing behavior.

Step 36 is complete as a docs-first checkpoint. Step 37 implements the approved narrow pack trust dry-run gate.

## Step 36 - Pack trust enforcement design and dry-run gate plan

Status: complete as docs-first design. No Rust source code changed.

Completed:

- [x] Defined enforcement states: `off`, `warn-only`, `dry-run-deny`, and `hard-deny`.
- [x] Defined Safe, Team, and Production mode behavior.
- [x] Defined effective source classification for built-in, project-local, and mixed-source pack material.
- [x] Defined built-in, project-local, unsigned local, invalid signature, expired signature, and expired trust-root behavior.
- [x] Defined future `pack_trust` config shape and safe defaults.
- [x] Defined future audit events and evidence artifacts for pack-load trust decisions.
- [x] Defined failure behavior when evidence or audit cannot be written.
- [x] Defined rollback behavior.
- [x] Listed test plan names only.
- [x] Preserved no runtime enforcement, no signing tools, no trust-root mutation, no pack install/update, no registry downloads, and no new action domains.

Step 37 has implemented the narrow dry-run-only pack-load trust gate. Step 38 adds gate-preview reporting. Step 39 adds gate-history reporting. Step 40 adds gate-status reporting. Step 41 adds gate-summary reporting. Hard-deny enforcement remains deferred until dry-run evidence is reviewed and explicitly approved.


## Step 37 pack-load trust dry-run gate

Status: implemented at checkpoint level; external Rust validation pending.

Step 37 moves pack trust from diagnostics-only into a narrow runtime dry-run gate. It evaluates effective loaded pack material before implemented Developer Pack runtime actions and records warnings or would-deny outcomes without blocking pack loading.

Completed:

- `pack_trust` config parsing with Safe/Team/Production defaults.
- effective source classification for `builtin`, `project_local`, and `project_local_mixed`.
- dry-run gate insertion before current Developer Pack runtime actions.
- evidence artifacts for warning and dry-run denial outcomes.
- audit events for `pack.trust.warning`, `pack.trust.dry_run_denied`, and `evidence.created`.
- fail-closed behavior when required dry-run gate evidence or audit cannot be written for project-local or mixed-source actions.

Hard-deny remains deferred.


## Step 38 - Pack load trust gate preview reporting

Status: implemented at checkpoint level; external Rust validation deferred.

Step 38 adds `gadgets pack trust gate-preview [--project <path>] [--operation <operation>] <pack>`. The command previews the configured runtime pack-load trust gate decision for an installed pack, including effective source classification, operation-specific Developer Pack Gadget material, configured enforcement, effective Step 37 enforcement, hard-deny deferral, signature coverage, and loaded Gadget sources.

The command writes diagnostic evidence and appends `pack.trust.gate.previewed` plus `evidence.created` audit events. It does not execute Gadgets, hard-deny pack loading, mutate trust roots, install packs, download packs, or run provider tools.


## Step 39 - Pack load trust gate history reporting

Status: implemented at checkpoint level; external Rust validation deferred.

Step 39 adds `gadgets pack trust gate-history [--project <path>] [--limit <n>]`. The command reads the local audit ledger and prints a focused history of pack-load trust gate warning, dry-run denial, preview, and future hard-denial events.

The command is read-only. It does not write evidence, append audit events, execute Gadgets, hard-deny pack loading, mutate trust roots, install packs, download packs, or run provider tools.

Hard-deny enforcement remains deferred until dry-run behavior and evidence are reviewed and explicitly approved.


## Step 40 - Pack trust gate status reporting

Status: implemented at checkpoint level; external Rust validation deferred.

Step 40 adds `gadgets pack trust gate-status [--project <path>]`. The command reads local configuration and prints a read-only summary of the configured trust-gate posture, including active runtime mode, configured and effective Step 37 enforcement states for Safe, Team, and Production, hard-deny deferral, evidence/audit requirements, installed packs, and the ledger path.

The command does not write evidence, append audit events, load packs, verify signatures, execute Gadgets, or hard-deny pack loading.

Hard-deny enforcement remains deferred until dry-run behavior and evidence are reviewed and explicitly approved.


## Step 41 - Pack trust gate summary reporting

Status: implemented at checkpoint level; external Rust validation deferred.

Step 41 adds `gadgets pack trust gate-summary [--project <path>]`. The command reads local configuration and the local audit ledger, counts pack-load trust gate preview, warning, dry-run denial, future hard-denial, and future pack-load denial events, and reports a review posture for future hard-deny discussions.

The command does not write evidence, append audit events, load packs, verify signatures, execute Gadgets, or hard-deny pack loading.

Hard-deny enforcement remains deferred until dry-run behavior, event summaries, and evidence are reviewed and explicitly approved.


## Step 42 - AI RMF alignment and governance profile design

Step 42 is complete as a docs/spec-only checkpoint. It adds a governance alignment model that maps current and future Gadgets controls to the NIST AI RMF Core functions: Govern, Map, Measure, and Manage.

Completed:

- [x] Added `docs/STEP_42_AI_RMF_ALIGNMENT_GOVERNANCE_PROFILE.md`.
- [x] Added `specs/AI_RMF_GOVERNANCE_PROFILE_SPEC.md`.
- [x] Defined current Gadgets controls that support Govern, Map, Measure, and Manage.
- [x] Defined future provider/model inventory, data exposure labels, AI incident classes, evidence artifacts, audit events, and posture values.
- [x] Preserved no-compliance-claim wording.

Boundary: Step 42 does not add runtime code, new CLI commands, provider authority, hard-deny enforcement, signing tools, trust-root mutation, pack install/update behavior, registry downloads, Linux admin behavior, database behavior, cloud behavior, deployment behavior, broader Git behavior, or compliance claims.


## Step 43 - Provider and model inventory design

Step 43 is complete as a docs/spec-only checkpoint. It defines the future provider/model inventory contract that supports AI RMF-style Map and Govern functions.

Step 43 adds:

- `docs/STEP_43_PROVIDER_MODEL_INVENTORY_DESIGN.md`
- `specs/PROVIDER_MODEL_INVENTORY_SPEC.md`
- a future config shape under `ai_risk.provider_model_inventory`
- provider inventory fields for status, network posture, credential environment variable name, approved modes, allowed data labels, and review status
- model profile inventory fields that map existing `model_profiles` entries to allowed task kinds, packs, Gadgets, runtime modes, and data labels
- future evidence, audit, report posture, and migration paths

Boundary: Step 43 does not add runtime code, CLI commands, provider calls, provider disablement, data exposure enforcement, compliance claims, credential storage, hard-deny enforcement, signing tools, trust-root mutation, pack install/update behavior, registry downloads, Linux admin behavior, database behavior, cloud behavior, deployment behavior, broader Git behavior, or provider-side action bypass.


## Step 43 pre-validation review and drift cleanup

Status: complete; ready for external Codex validation.

This checkpoint reviews the Step 43 tree for validation readiness, updates active planning docs away from deferred-validation language, adds a Codex validation prompt, and regenerates `FILE_MANIFEST.txt`.

Next action:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Boundary: no runtime code, CLI behavior, provider behavior, hard-deny enforcement, signing tools, trust-root mutation, pack install/update behavior, registry downloads, Linux admin behavior, database behavior, cloud behavior, deployment behavior, broader Git behavior, or provider-side action bypass are added by this checkpoint.
