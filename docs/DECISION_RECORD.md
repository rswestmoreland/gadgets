# Decision Record

Date: 2026-05-12

## DR-001 - Rust core runtime

Decision: Use Rust for the safety-critical runtime.

Reason: The runtime enforces authority boundaries, policy, evidence, audit, and provider/tool isolation.

## DR-002 - CLI-first MVP

Decision: Start with a local CLI.

Reason: A CLI proves the safety model without requiring a service platform, UI, or distributed deployment.

## DR-003 - Provider-neutral model layer

Decision: Use provider adapters.

Reason: OpenAI, Anthropic, local models, and future providers should be interchangeable behind the Gadget runtime.

## DR-004 - Mock provider first

Decision: Implement mock provider before a live provider.

Reason: The runtime and policy model must be testable without live model behavior.

## DR-005 - YAML manifests and JSONL audit

Decision: Use YAML for human-authored manifests/config and JSONL for append-only event streams.

Reason: YAML is readable. JSONL is simple, appendable, and easy to verify.

## DR-006 - Built-in policy first

Decision: Use deterministic built-in policy checks before policy-as-code.

Reason: Avoid Kubernetes-like complexity in the first release.

## DR-007 - Safe Mode default

Decision: Safe Mode is default.

Reason: The default experience must prevent production writes, destructive actions, and secret exposure.

## DR-008 - Developer Pack first

Decision: Build Developer Pack before server admin, database, cloud, or deployment packs.

Reason: Developer automation provides useful workflows with lower blast radius.

## DR-009 - Linux Server Admin as pack family

Decision: Add Linux Server Admin Observe Pack before Change Pack.

Reason: Server administration is powerful and common, but mutation must be tightly gated.

## DR-010 - No generic root-shell Gadget

Decision: Do not build a broad shell/root Gadget.

Reason: It undermines the entire least-privilege Gadget model.

## DR-011 - Approval required for file writes in v0.1

Decision: All writes require explicit approval in the first release.

Reason: This is the simplest and safest default.

## DR-012 - Markdown docs always maintained

Decision: Every meaningful build/design step updates Markdown documentation.

Reason: Docs preserve design continuity and can later support user-facing and marketing material.

## License

Decision: Gadgets Framework is dual-licensed under MIT OR Apache-2.0.

Author: Richard S. Westmoreland <dev@rswestmore.land>

Copyright 2026 Richard S. Westmoreland


## DR-013 - Validated Developer MVP baseline

Decision: Treat commit `c5fbd78` as the current validated Developer MVP baseline.

Reason: The external Rust validation flow passed end-to-end: `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build --release`.

Scope: This validates the current local Developer MVP plus guarded remote GitHub PR creation path. It does not imply support for arbitrary shell, Git push/fetch/pull/merge/rebase, Linux admin mutation, database/cloud/deployment behavior, or provider-side tool execution bypass.

## DR-014 - Developer MVP alpha packaging

Decision: Treat `docs/DEVELOPER_MVP_ALPHA.md` as the primary alpha guide for the current validated Developer MVP.

Reason: The Developer workflow is now implemented, externally validated, and broad enough that users need one concise guide covering capabilities, non-goals, safe configuration, evidence, audit, troubleshooting, and known limitations.

Scope: This does not add runtime behavior. It documents the current validated local-first workflow and optional guarded GitHub PR creation path.


## DR-015 - Remote PR safety hardening

Decision: Keep remote PR creation disabled by default and dry-run by default, with configured base/head branch constraints and duplicate-open-PR handling before any GitHub mutation.

Reason: Remote PR creation crosses a network and repository boundary. Dry-run mode, branch allowlists, duplicate handling, and redacted API evidence reduce accidental external mutation while preserving the explicit review workflow.

Scope: This does not add Git push, fetch, pull, merge, rebase, checkout, switch, GitLab support, provider-side tool execution, arbitrary shell, Linux admin behavior, database behavior, cloud behavior, or deployment behavior.

## Step 25 shared redaction helper

Decision: centralize best-effort evidence redaction in `gadgets-tools` before adding more integrations.

Reason: duplicated local redaction helpers made it easy for future providers to drift. A shared helper gives the Developer MVP a consistent baseline for stdout, stderr, Git output, PR body text, and remote API response evidence.

Limit: this remains best-effort redaction, not full DLP or a complete secret scanner.

## DR-016 - Pack trust and signing design

Decision: Pack trust/signing is defined as a supply-chain eligibility check, not as runtime action authority.

Reason: Third-party and Team/Production packs will need deterministic provenance and integrity checks, but signed packs must still remain inside the runtime guardrails. A signature can prove publisher and content integrity; it cannot grant arbitrary execution or bypass policy.

Locked direction:

- Pack identity includes manifest and content hashes.
- Signed packs should use deterministic content manifests and detached signature records.
- Recommended primitives are SHA-256 and Ed25519.
- Safe mode can allow explicit unsigned local packs with audit warnings.
- Team mode should require signed non-built-in packs unless an explicit team exception exists.
- Production mode should fail closed for unsigned or invalid packs.

Status: design locked in Step 26. Non-enforcing trust inspection scaffold added in Step 27. Enforcement code remains future work.


## Step 27 - Pack trust inspection scaffold

Decision: add a non-enforcing `gadgets pack trust check [--project <path>] <pack>` diagnostic path before cryptographic verification or enforcement.

Rationale: pack trust should be observable before it becomes an execution gate. The inspection scaffold lets local users see whether a pack is built-in or project-local, whether optional content/signature metadata exists, whether basic metadata hashes line up, and whether local trust roots are present.

Boundaries:

- no signature enforcement
- no signing tools
- no trust-root mutation
- no registry downloads
- no pack install/update commands
- no Team/Production enforcement
- no Gadget execution

Status: complete at checkpoint/code level; Rust validation should be rerun after Step 27.
## Step 28 - Trust root inspection scaffold

Decision: add a non-enforcing `gadgets pack trust roots [--project <path>]` diagnostic path before trust-root mutation, signature verification, or Team/Production enforcement.

Rationale: trust-root files should become observable before they become security gates. The inspection scaffold lets users see whether `.gadgets/trust/trusted_publishers.yaml` exists, whether it parses, and what publisher metadata it declares without changing trust state.

Boundary: the command does not verify cryptographic signatures, enforce trust, mutate trust roots, install packs, download packs, execute Gadgets, or change Safe/Team/Production runtime behavior.



## Step 29 - Pack trust evidence and audit design

Decision: define pack trust audit events and evidence artifacts before adding enforcement.

Rationale: pack trust will eventually control whether packs are eligible to load. That decision must be explainable, reviewable, and auditable before it becomes a hard runtime gate.

Locked outcome: future pack trust work should use stable events such as `pack.trust.checked`, `pack.trust.allowed`, `pack.trust.denied`, `pack.signature.verified`, `pack.signature.failed`, `trust.root.loaded`, `trust.root.rejected`, and `pack.load.denied`. Future evidence should include trust decisions, pack identity, manifest/content hashes, signature summaries, trust-root summaries, findings, and runtime mode, without copying private keys or secret-bearing configs into evidence.

Boundary: Step 29 does not implement cryptographic verification, enforcement, signing tools, trust-root mutation, registry downloads, pack install/update, or Gadget execution changes.


## Step 30 - Pack trust diagnostic evidence emission

Decision: add diagnostic evidence bundles and audit events to `gadgets pack trust check` and `gadgets pack trust roots`.

Rationale: pack trust decisions should be reviewable before enforcement is introduced. Diagnostic output now has the same evidence/audit shape as other meaningful Gadgets work without turning trust checks into enforcement gates.

Boundary: Step 30 does not implement cryptographic signature verification, pack trust enforcement, signing tools, trust-root mutation, registry downloads, pack install/update, or Gadget execution changes.


## Step 31 - Pack trust policy preview

Decision: add a non-enforcing `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` diagnostic path.

Rationale: pack trust policy should be explainable before it becomes authoritative. The preview command shows how a pack would be treated under Safe, Team, or Production policy while preserving the current non-enforcing loading behavior.

Locked outcome: built-in packs preview as trusted. Safe Mode project-local packs preview as allowed with warnings. Team and Production previews require verified signatures and trust-root matches. The preview remains non-enforcing and does not yet enforce pack loading.

Boundary: Step 31 does not implement cryptographic verification, signing tools, trust-root mutation, pack install/update, registry downloads, or pack-load enforcement.

## Step 32 - Signature metadata diagnostics remain non-cryptographic

Decision: add a diagnostic-only signature metadata verification scaffold before implementing cryptographic verification.

Rationale: signature metadata, trust-root references, evidence, audit, and CLI behavior should be stable before enforcement is introduced.

Boundary: Step 32 does not verify signatures cryptographically and does not enforce pack trust decisions.

## Step 33 - Cryptographic verification byte contract finalized

Decision: finalize the byte-level design for future pack cryptographic signature verification before implementing code.

Locked outcome:

- version 1 signatures use Ed25519
- version 1 hashes use SHA-256
- `manifest_sha256` is the SHA-256 of raw `pack.yaml` bytes
- `contents_sha256` is the SHA-256 of raw `pack.contents.yaml` bytes
- the signature payload is a deterministic ASCII/UTF-8 line-based payload beginning with `gadgets-pack-signature-v1`
- the content manifest must verify every listed file hash before a pack can be trusted as signed
- trust-root matching requires publisher, key id, algorithm, allowed pack id, and unexpired trust-root metadata

Boundary: Step 33 does not implement cryptographic verification, signing tools, trust-root mutation, pack install/update, registry downloads, or pack-load enforcement.


## Step 34 - Ed25519 signature diagnostics implemented

Decision: add real Ed25519 verification to the diagnostic `gadgets pack trust signature` path before adding enforcement.

Rationale: the cryptographic path should be observable and evidence-backed before it becomes a pack-load gate.

Locked outcome:

- Ed25519 verification uses the deterministic `gadgets-pack-signature-v1` payload.
- Public keys and signatures are base64 without line breaks.
- The verifier recomputes raw-byte SHA-256 over `pack.yaml` and `pack.contents.yaml`.
- The verifier checks every listed content file hash before accepting a signed pack diagnostic.
- The verifier requires matching publisher, key id, algorithm, allowed pack id, and non-expired trust-root metadata.
- The diagnostic records verification findings as evidence and audit, but does not enforce pack loading.

Boundary: Step 34 does not add signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, or Gadget execution behavior changes.

## Step 35 - Signature-aware pack trust policy preview

Decision: update `gadgets pack trust preview` to consume real signature diagnostics from the Ed25519 verification path.

Rationale: policy preview should match the same signed-pack inputs that future Team/Production enforcement will use. This makes future enforcement explainable before it becomes authoritative.

Locked outcome: built-in packs remain trusted in all modes. Safe Mode allows project-local development packs with warnings. Team and Production previews allow only valid trusted signatures diagnostically. The preview remains non-enforcing and does not change pack loading behavior.

Boundary: Step 35 does not add signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, or Gadget execution behavior changes.
