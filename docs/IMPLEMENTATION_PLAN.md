# Implementation Plan

Date: 2026-05-13

## Build principle

Build the runtime safety skeleton before adding powerful Gadgets.

The goal is a trustworthy, auditable authority boundary where provider output remains untrusted and the Gadgets runtime authorizes every meaningful action.

## Current source and validation status

```text
current source checkpoint: Step 35 - pack trust policy preview with signature results
validation status: external Rust validation pending after post-Step-22 source changes
last validated commit: c5fbd78
```

Validation passed on the last validated baseline:

```text
cargo fmt --check                                      PASS
cargo check                                           PASS
cargo test                                            PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo build --release                                 PASS
```

Rust/Cargo versions reported by Codex:

```text
rustc 1.89.0 (29483883e 2025-08-04)
cargo 1.89.0 (c24e10642 2025-06-23)
```

## Completed implementation steps

### Step 1 - Skeleton and docs

Status: complete.

- Root README.
- Architecture docs.
- Baseline docs.
- Decision record.
- Roadmap.
- Contract specs.
- Built-in pack placeholders.
- Minimal Rust workspace skeleton.

### Step 2 - Core data types and manifest parsing

Status: complete and validated.

- Gadget and Pack manifest structs.
- Capability names.
- Permission levels.
- Zones and boundaries.
- Handoffs.
- Action requests.
- Policy decisions.
- Evidence metadata.
- Audit metadata.
- Validation reports and errors.

### Step 3 - Local project init

Status: complete and validated.

Implemented:

```bash
gadgets init [path]
```

Creates local `.gadgets/` project state with Safe Mode, mock provider defaults, Developer Pack selection, denied secret/protected paths, approval posture, test command config, Git settings, and remote PR settings disabled by default.

### Step 4 - Append-only audit ledger

Status: complete and validated.

Implemented:

```bash
gadgets ledger show [project-root-or-ledger-path]
gadgets ledger verify [project-root-or-ledger-path]
```

Includes JSONL events, SHA-256 event hashes, previous-event hash chaining, verification, and append refusal when the existing ledger is invalid.

### Step 5 - Evidence bundles

Status: complete and validated.

Implemented:

```bash
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
```

Includes per-run evidence directories, `bundle.yaml`, `summary.md`, optional artifacts, artifact hashes, and bundle metadata hashes.

### Step 6 - Deterministic policy engine v0.1

Status: complete and validated.

Implemented checks for declared capabilities, permission levels, tool allowlists, zones, filesystem paths, denied paths, readable/writable boundaries, Safe Mode/Team Mode restrictions, approval-required mutating actions, allowlisted test execution, local Git status, protected local branch creation, approved local commits, PR body generation, and guarded remote PR creation.

### Step 7 - Observe-only Filesystem Read Gadget

Status: complete and validated.

Implemented observe-only repo inspection through:

```bash
gadgets ask <request>
```

The Filesystem Read slice routes file search/read actions through policy, writes evidence, and appends audit events. It does not modify files or execute commands.

### Step 8 - Mock provider and Coordinator stub

Status: complete and validated.

Added deterministic `MockProvider` and a Coordinator stub that emits structured handoffs. Provider output remains untrusted and must pass runtime validation.

### Step 9 - Config loading and provider profile selection

Status: complete and validated.

`gadgets ask` loads `.gadgets/config.yaml`, selects `default_model_profile`, validates provider support, and passes configured runtime mode into policy evaluation.

### Step 10 - Pack and Gadget manifest loading

Status: complete and validated.

Implemented project-local and built-in pack/Gadget manifest loading, plus:

```bash
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
```

### Step 11 - Pack validation

Status: complete and validated.

Implemented:

```bash
gadgets pack validate [--project <path>] [--strict] [pack]
```

Validates pack manifests, declared Gadget manifests, missing/invalid Gadgets, manifest name mismatches, and pack highest-permission constraints. Later pack trust diagnostics are covered in Steps 27 and 28.

### Step 12 - OpenAI provider adapter

Status: complete and validated.

Added `OpenAiProvider` behind the provider trait. OpenAI can propose structured Coordinator handoffs but cannot execute tools, read files directly, approve actions, or mutate state.

### Step 13 - Anthropic provider adapter

Status: complete and validated.

Added `AnthropicProvider` behind the provider trait. Anthropic can propose structured Coordinator handoffs but cannot execute tools, read files directly, approve actions, or mutate state.

### Step 14 - Patch Writer plan-only mode

Status: complete and validated.

Patch Writer can produce a `proposed.patch` evidence artifact through a policy-checked plan action. It does not apply patches, write files, run commands, stage files, commit, or open PRs.

### Step 15 - Approval record scaffolding

Status: complete and validated.

Implemented:

```bash
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <YYYY-MM-DDTHH:MM:SSZ>]
gadgets approval approve [--project <path>] <approval-request-id> <approver>
gadgets approval show [--project <path>] <approval-request-id>
gadgets approval verify [--project <path>] <approval-request-id>
gadgets approval id-for-run <run-id>
```

Approvals bind to the exact proposed patch hash, deterministic scope hash, and strict UTC expiration when provided.

### Step 16 - Approved local patch application

Status: complete and validated.

Implemented:

```bash
gadgets patch apply [--project <path>] <approval-request-id>
```

Patch application requires a valid approval request, valid approval record, matching scope hash, matching proposed patch SHA-256, parseable supported unified diff, allowed target paths, and all target file changes prepared before any file write.

### Step 17 - Allowlisted Test Runner

Status: complete and validated.

Implemented:

```bash
gadgets test run [--project <path>] <test-command-name>
```

Runs only named test commands configured in `.gadgets/config.yaml`. It rejects unknown commands, empty command names, unsafe working directories, parent traversal, shell metacharacters, and model/user-supplied raw command strings.

### Step 18a - Local Git status

Status: complete and validated.

Implemented:

```bash
gadgets git status [--project <path>]
```

Runs only the fixed local Git status command selected by the runtime.

### Step 18b - Protected local branch creation

Status: complete and validated.

Implemented:

```bash
gadgets git branch create [--project <path>] <branch-name>
```

Runs only `git branch <validated-branch-name>` after branch-name validation and protected-branch checks. It does not checkout or switch branches.

### Step 18c - Approved local commit scaffolding

Status: complete and validated.

Implemented:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

Verifies a scoped patch approval, rejects protected current branches and detached HEAD, rejects preexisting staged changes, stages only approved patch files, verifies the staged set, and creates one local commit.

### Step 19 - Local PR body generation

Status: complete and validated.

Implemented:

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

Generates reviewable local PR Markdown evidence from verified approval context plus optional test and commit evidence references.

### Step 20 - Local Developer MVP hardening

Status: complete and validated.

Completed:

- approval expiration format locked to strict UTC RFC3339 without fractional seconds
- invalid expiration format rejected at approval request creation
- expired approval requests rejected before approval recording
- expired approvals rejected by approval verification
- CLI help updated for `--expires-at`
- local Developer MVP walkthrough added

### Step 21 - Guarded remote PR creation

Status: complete and validated.

Implemented:

```bash
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

Creates one GitHub pull request only when explicitly enabled by config, backed by verified approval, verified local PR body evidence, deterministic policy, and a token loaded from the configured environment variable. It does not push branches, fetch, pull, merge, rebase, checkout, switch, run shell, call provider tools, apply patches, run tests, or perform Linux admin/database/cloud/deployment actions.

### Step 21 validation checkpoint

Status: complete and validated.

The external Rust validation flow passed on commit `c5fbd78`.

### Step 22 - Post-validation baseline reconciliation

Status: complete and validated.

Purpose:

- record the validated `c5fbd78` baseline
- reconcile README, roadmap, implementation plan, walkthrough, and progress docs
- remove stale active-doc claims that Rust validation remains pending
- preserve the unsupported behavior boundaries

### Step 23 - Developer MVP alpha packaging

Status: complete in this checkpoint.

Purpose:

- add `docs/DEVELOPER_MVP_ALPHA.md`
- explain what the alpha can do today
- explain what the alpha intentionally cannot do
- provide safe local config examples
- provide sample test command and remote PR config snippets
- provide a complete alpha workflow
- document evidence and audit behavior
- document troubleshooting and known limitations

### Step 24 - Remote PR safety hardening

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Completed:

- added `git.remote_pr.dry_run`, defaulting to true
- kept remote PR creation disabled by default
- added `git.remote_pr.allowed_base_branches`
- added `git.remote_pr.allowed_head_prefixes`
- added `git.remote_pr.duplicate_strategy` with `fail` and `reuse`
- added branch allowlist checks before remote PR mutation
- added duplicate-open-PR lookup before create when dry-run is false
- made dry-run skip token reading and skip GitHub mutation
- improved remote API response redaction before evidence write
- preserved no Git push, fetch, pull, merge, rebase, checkout, switch, shell, provider tool, patch apply, test run, Linux admin, database, cloud, or deployment action

### Step 25 - Secret/output redaction hardening

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Completed:

- added shared redaction helper in `crates/gadgets-tools/src/redaction.rs`
- centralized best-effort line redaction and UTF-8-safe truncation
- applied shared helper to Test Runner stdout/stderr evidence
- applied shared helper to Git status, branch, and commit output evidence
- applied shared helper to local PR body text and referenced evidence summaries
- applied shared helper to remote PR API response evidence
- added tests for common secret-like patterns and truncation behavior
- documented that redaction is best-effort and not full DLP


### Step 26 - Pack trust/signing design

Status: complete as a docs-first design checkpoint. Enforcement code is not implemented yet.

Completed:

- defined pack identity as id, version, publisher, source, manifest hash, and content hash
- defined deterministic content manifest design
- defined detached signature record design
- selected SHA-256 and Ed25519 as recommended primitives
- defined local trust-root file shape
- defined Safe/Team/Production mode behavior
- defined verification outcomes, audit events, and evidence artifacts
- added `specs/PACK_TRUST_SIGNING_SPEC.md`

### Step 27 - Pack trust inspection scaffold

Status: complete at checkpoint/code level. Rust validation should be rerun after this code change.

Implemented:

```text
gadgets pack trust check [--project <path>] <pack>
```

The command inspects pack source and trust metadata, reports a diagnostic trust decision shape, and avoids mutations. It does not implement cryptographic signature verification, signing tools, registry downloads, pack install/update behavior, or Team/Production enforcement.

## Deferred / future work

- Developer MVP usability polish.
- Documentation Gadget executable behavior.
- Dependency Gadget plan-only behavior.
- Live remote PR provider validation.
- Pack trust enforcement and signing tools.
- Linux Server Admin Observe Pack implementation.
- Linux Server Admin Change Pack implementation.
- Database, cloud, and deployment packs.
- Team workflows.
- Arbitrary shell remains intentionally excluded.


## Step 28 - Trust root inspection scaffold

Status: complete at checkpoint/code level. Rust validation was not rerun after this source change.

Implemented command:

```text
gadgets pack trust roots [--project <path>]
```

Scope:

- inspect `.gadgets/trust/trusted_publishers.yaml` when present
- report existence, parse status, version, publisher count, publisher summaries, and findings
- keep the command diagnostic only

Non-goals preserved:

- no signature verification
- no trust enforcement
- no trust-root mutation
- no signing tools
- no pack install/update behavior
- no registry downloads
- no Gadget execution


### Step 29 - Pack trust evidence and audit design

Status: complete at design level.

Step 29 defines future audit events and evidence artifacts for pack trust checks, trust-root loading, signature verification, and pack-load denials. It does not add runtime enforcement, cryptographic verification, signing tools, trust-root mutation, pack install/update, registry downloads, or Gadget execution changes.

Future implementation should add evidence/audit emission for diagnostics before adding Team or Production mode enforcement.


## Step 31 - Pack trust policy preview

Status: implemented as non-enforcing diagnostics.

Command:

```bash
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
```

The command previews how a pack would be treated under Safe, Team, or Production policy, writes evidence, and appends audit events. As of Step 35, it consumes real signature diagnostic results from the Ed25519 verification path. It does not enforce trust decisions or change pack loading.

## Step 32 - Signature metadata verification scaffold

Step 32 adds:

```bash
gadgets pack trust signature [--project <path>] <pack>
```

The command validates signature metadata shape and trust-root references, writes diagnostic evidence, and appends audit events. Step 34 extends this command with real Ed25519 verification while keeping it non-enforcing.

## Step 33 - Cryptographic verification design finalization

Status: complete as a docs-first design checkpoint.

Step 33 finalizes the byte-level design for future real pack signature verification. It locks Ed25519, SHA-256, raw-byte file hashing, the deterministic `gadgets-pack-signature-v1` payload, content manifest verification, trust-root matching, denial mappings, evidence artifacts, audit events, and rollout order.

No cryptographic verification code, signing tools, trust-root mutation, pack install/update, registry download, or pack-load enforcement was added.

Step 34 adds real Ed25519 verification to `gadgets pack trust signature` diagnostics only, while keeping the command non-enforcing.


## Step 34 - Ed25519 verification diagnostics

Step 34 adds real cryptographic verification to the diagnostic `gadgets pack trust signature` path only.

Implemented behavior:

- recompute raw-byte SHA-256 for `pack.yaml` and `pack.contents.yaml`
- validate content manifest entries and listed file hashes
- require `pack.yaml` to be listed in signed contents
- reject `pack.signature.yaml` in signed contents
- match publisher, key id, algorithm, and allowed pack id against trust roots
- check signature and trust-root expiration metadata
- build the deterministic `gadgets-pack-signature-v1` payload
- verify Ed25519 signatures with base64 public keys/signatures
- write diagnostic evidence and audit events

Boundary: Step 34 does not enforce pack trust, mutate trust roots, add signing tools, install packs, download packs, enable Team/Production enforcement, execute Gadgets, or add arbitrary shell.

Step 35 updates pack trust policy preview to consume real signature diagnostic results while remaining non-enforcing.

## Step 35 - Pack trust policy preview with signature results

Step 35 updates `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` to consume signature diagnostics from the Ed25519 verification path.

Safe Mode continues to allow local development packs with warnings. Team and Production previews allow only valid trusted signatures diagnostically. The command remains non-enforcing and does not change pack loading behavior.

Next recommended step: Step 36 - pack trust enforcement design and dry-run gate plan.
