# Gadgets Framework

Gadgets Framework is a safety-first, vendor-neutral framework for purpose-built AI workers called Gadgets.

A Gadget is a least-privilege AI worker that operates inside fixed capability zones and collaborates only through policy-enforced handoffs.

## Core rule

Models may reason, propose, summarize, and request actions.

Only the Gadgets runtime may authorize and execute actions.

Provider SDK behavior, prompts, model tool-calling, and agent handoff features are useful integration surfaces, but they are not the final security boundary.

## Current source and validation status

Current source checkpoint:

```text
Step 35 - pack trust policy preview with signature results
validation status: external Rust validation pending after post-Step-22 source changes
```

Last externally validated baseline:

```text
gadgets-main.zip
validated commit: c5fbd78
validation date: 2026-05-13
```

External Rust validation passed end-to-end on the last validated baseline:

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

## Initial product shape

The first implementation is a CLI-first local runtime focused on safe developer automation.

The current Developer Pack workflow can inspect a repository, propose a patch, create and approve a scoped patch approval record, apply the exact approved patch when policy and hash checks pass, run a named configured test command, record local Git status evidence, create a validated non-protected local branch, create one local commit from approved patch files on a non-protected branch, generate a reviewable local PR body Markdown artifact, and optionally create one GitHub pull request when remote PR creation is explicitly enabled in config.

For a single alpha-oriented guide, see `docs/DEVELOPER_MVP_ALPHA.md`. It explains what the MVP can do today, what it intentionally cannot do, safe configuration examples, the command walkthrough, troubleshooting, evidence, audit, and known limitations.

Expected local flow:

```bash
gadgets ask "Review this repo and explain how it is structured."
gadgets ask "Propose a patch..."
gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z
gadgets approval approve <approval-request-id> <approver>
gadgets patch apply <approval-request-id>
gadgets test run <test-command-name>
gadgets git status
gadgets git branch create <branch-name>
gadgets git commit approved-patch <approval-request-id>
gadgets git pr body <approval-request-id>
```

Optional guarded remote PR creation:

```bash
gadgets git pr create <approval-request-id> --body-run <pr-body-run-id> --head <branch> --base <branch>
```

Remote PR creation is disabled by default. It requires explicit config, a verified and unexpired approval, local PR body evidence, configured base/head branch constraints, duplicate-open-PR handling, deterministic policy approval, and a configured GitHub token environment variable when dry-run mode is disabled. It does not push branches; the head branch must already exist remotely. Dry-run mode is enabled by default in generated config.

## Safety boundaries

Implemented boundaries:

- Provider output is treated as an untrusted structured request.
- Runtime policy remains the action authority.
- Evidence and audit are required for meaningful work.
- Patch application does not run tests, Git commands, PR behavior, shell commands, Linux admin actions, database actions, cloud actions, or deployment actions.
- Test running accepts only named commands from `.gadgets/config.yaml`.
- Test commands run through `std::process::Command` without `sh -c`.
- Git status uses one fixed observe command.
- Local branch creation uses one fixed branch command after branch-name validation and protected-branch checks.
- Local commit creation requires verified approval scope and stages only approved patch files.
- Local PR body generation writes reviewable Markdown evidence.
- Remote PR creation is GitHub-only, disabled by default, config-gated, dry-run by default, branch-constrained, and duplicate-aware.
- Evidence output redaction is centralized and best-effort for stdout, stderr, Git output, PR body text, and remote API responses.

Still intentionally not implemented:

- arbitrary shell execution
- generic root-shell Gadget
- provider-side tool execution bypass
- Git push, fetch, pull, merge, or rebase
- Git checkout or switch
- remote branch creation
- Linux server administration actions
- database, cloud, or deployment behavior
- full secret scanner or DLP model; current redaction is best-effort only
- pack trust enforcement and signing tools; Steps 26-35 define design, inspection, trust-root diagnostics, diagnostic evidence/audit emission, policy preview, signature metadata diagnostics, byte-level verification design, non-enforcing Ed25519 signature diagnostics, and signature-aware policy preview only


## Pack trust status

Pack trust is being added in staged, non-enforcing steps.

Implemented diagnostics:

```bash
gadgets pack trust check [--project <path>] <pack>
gadgets pack trust roots [--project <path>]
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
gadgets pack trust signature [--project <path>] <pack>
```

Current boundary: these commands report trust metadata, trust-root diagnostics, future policy outcomes, and signature verification diagnostics. They write diagnostic evidence bundles and append audit events. `gadgets pack trust signature` performs Ed25519 verification when signed pack metadata and matching trust-root public keys are available. None of the pack trust commands enforce signed-pack requirements, edit trust roots, install packs, download packs, or execute Gadgets. Trust enforcement remains unimplemented.

## License and author

Gadgets Framework is dual-licensed under MIT OR Apache-2.0, at your option.

- MIT License: see `LICENSE-MIT`
- Apache License, Version 2.0: see `LICENSE-APACHE`
- Dual-license summary: see `LICENSE.md`

Author: Richard S. Westmoreland <dev@rswestmore.land>

Copyright 2026 Richard S. Westmoreland

## Repository layout

```text
docs/       Architecture, decisions, implementation plan, roadmap.
specs/      Contract specs for manifests, capabilities, zones, handoffs, evidence, audit, providers, packs, and pack trust/signing.
crates/     Rust workspace crates.
packs/      Built-in Gadget pack manifests and Gadget manifests.
examples/   Example projects and local `.gadgets/` configuration.
```

## Current implemented commands

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

## Allowlisted test command workflow

The Test Runner runs only named test commands configured in `.gadgets/config.yaml`. It does not accept arbitrary command strings from a model response or from free-form user input. It launches commands directly without `sh -c`, checks `test.run` through deterministic policy, captures stdout/stderr/exit status/duration, writes evidence, and appends audit events.

Example config:

```yaml
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
    timeout_seconds: 300
```

Run by name:

```bash
gadgets test run cargo_test
```

## Local Git workflow

Git status:

```bash
gadgets git status [--project <path>]
```

Runs only:

```text
git status --short --branch --untracked-files=normal
```

Local branch creation:

```bash
gadgets git branch create [--project <path>] <branch-name>
```

Runs only:

```text
git branch <validated-branch-name>
```

Approved local commit:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

The commit provider verifies approval, rejects detached HEAD and protected current branches, rejects preexisting staged changes, stages only approved patch files, verifies the staged set, and creates one local commit.

Local PR body generation:

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

This writes local Markdown evidence only.

Guarded remote PR creation:

```bash
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

This is disabled by default and currently supports a single GitHub PR creation API call. It does not push branches.

## Approval expiration

Approval expiration uses strict UTC RFC3339 without fractional seconds:

```text
YYYY-MM-DDTHH:MM:SSZ
```

Expiration is validated when the request is created, rejected if already expired when approval is recorded, and checked again by approval verification. Patch apply, approved local commit, local PR body generation, and guarded remote PR creation all rely on approval verification.

## Pack trust/signing status

Step 26 defines the pack trust and signing model in `specs/PACK_TRUST_SIGNING_SPEC.md`.

The design locks the future approach for:

- pack identity
- content manifests
- detached signature records
- local trust roots
- Safe/Team/Production mode behavior
- verification outcomes
- audit and evidence expectations

Step 27 adds a non-enforcing pack trust inspection scaffold:

```bash
gadgets pack trust check [--project <path>] <pack>
```

The command reports whether a pack is built-in or project-local, whether optional `pack.contents.yaml` and `pack.signature.yaml` metadata are present, basic hash cross-check findings when metadata exists, and whether local trust roots are present. It does not verify cryptographic signatures, enforce signed-pack requirements, mutate trust roots, install packs, download packs, or execute Gadgets.

Step 28 adds a non-enforcing trust-root inspection scaffold:

```bash
gadgets pack trust roots [--project <path>]
```

The command reports whether `.gadgets/trust/trusted_publishers.yaml` exists, whether it parses, its version, configured publisher summaries, and diagnostic findings. It does not verify signatures, enforce trust, mutate trust roots, install packs, download packs, or execute Gadgets.

Step 30 adds diagnostic evidence and audit emission for both commands. Each run writes a normal evidence bundle under `.gadgets/runs/<run-id>/evidence` and appends audit events to `.gadgets/ledger/events.jsonl`.

Pack trust check evidence includes:

- `pack_trust_decision.txt`
- `pack_identity.yaml`
- `pack_manifest_hash.txt`
- `pack_contents_summary.txt`
- `pack_signature_summary.yaml`
- `trust_root_summary.txt`
- `trust_findings.txt`
- `policy_mode.txt`

Trust-root inspection evidence includes:

- `trust_root_path.txt`
- `trust_root_summary.yaml`
- `trusted_publishers_summary.txt`
- `trust_root_findings.txt`

Step 31 adds non-enforcing policy preview:

```bash
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
```

The command reports how the pack would be treated under the selected mode. As of Step 35, it consumes the real signature diagnostic result from `gadgets pack trust signature`. Safe Mode preview allows local development packs with warnings when signatures are missing or invalid. Team and Production previews allow only valid trusted signatures diagnostically. The preview command is still non-enforcing and does not change pack loading. Preview evidence includes `pack_trust_policy_preview.txt`, `pack_identity.yaml`, `pack_manifest_hash.txt`, `pack_trust_decision.txt`, `signature_policy_inputs.txt`, `trust_findings.txt`, and `policy_mode.txt`.


## Step 35 signature-aware policy preview

Step 35 updates `gadgets pack trust preview` so Safe, Team, and Production previews consume the same signature metadata, content-manifest, trust-root, expiration, and Ed25519 verification results produced by `gadgets pack trust signature`.

The preview remains diagnostic only. It does not enforce pack loading, mutate trust roots, install packs, download packs, execute Gadgets, or enable Team/Production gates.

## Recommended next step

Proceed with the next design or hardening checkpoint before enabling pack trust enforcement. External Rust validation should still be rerun before a release tag because Steps 24, 25, 27, 28, 30, 31, 32, 34, and 35 include Rust source changes after the last validated baseline.

## Step 32 - Signature metadata diagnostics

Step 32 adds a diagnostic-only signature metadata command:

```bash
gadgets pack trust signature [--project <path>] <pack>
```

The command checks `pack.signature.yaml` metadata shape, strict UTC timestamp format, pack identity/hash references, local trust-root publisher/key references, content manifest file hashes, and Ed25519 signatures when signed metadata and matching trust-root public keys are available. It writes evidence and audit events, but it does not enforce signatures, mutate trust roots, install packs, download packs, or execute Gadgets.

## Step 33 cryptographic verification design

Step 33 finalizes the design for real pack signature verification but does not add cryptographic verification code or enforcement.

Locked version 1 choices:

- Ed25519 signatures
- SHA-256 hashes
- lowercase hex digests
- base64 public keys and signatures without line breaks
- strict UTC timestamps without fractional seconds
- raw-byte SHA-256 over `pack.yaml`
- raw-byte SHA-256 over `pack.contents.yaml`
- deterministic line-based `gadgets-pack-signature-v1` payload

The diagnostic verifier validates `pack.contents.yaml` by checking every listed file hash, then verifies the detached `pack.signature.yaml` signature against a matching publisher/key in `.gadgets/trust/trusted_publishers.yaml`.

Step 33 does not add signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, or Gadget execution behavior changes.


## Step 34 Ed25519 verification diagnostics

Step 34 adds real Ed25519 signature verification to the diagnostic `gadgets pack trust signature` path only.

The command now verifies:

- raw-byte SHA-256 over `pack.yaml`
- raw-byte SHA-256 over `pack.contents.yaml`
- every listed file hash in `pack.contents.yaml`
- required `pack.yaml` content-manifest entry
- absence of `pack.signature.yaml` from signed content entries
- sorted, unique, safe relative content paths
- strict UTC signature and trust-root expiration metadata
- matching publisher, key id, algorithm, and allowed pack id in `.gadgets/trust/trusted_publishers.yaml`
- Ed25519 signature over the deterministic `gadgets-pack-signature-v1` payload

Step 34 remains diagnostic and non-enforcing. It does not add signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, or Gadget execution behavior changes.
