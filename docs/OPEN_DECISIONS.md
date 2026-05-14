# Open Decisions

Date: 2026-05-13

## License

Status: closed.

Gadgets Framework is dual-licensed under MIT OR Apache-2.0.

Author and copyright notice:

```text
Richard S. Westmoreland
dev@rswestmore.land
Copyright 2026 Richard S. Westmoreland
```

Repository license files:

- `LICENSE.md`
- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `AUTHORS.md`
- `COPYRIGHT.md`

## Validation baseline

Status: closed for Step 35 source baseline; pending for Step 37 through Step 41 source changes. Steps 42 and 43 are docs/spec only.

The current externally validated source baseline is commit `14b0a4f`. The full external Rust validation flow passed on Rust/Cargo 1.89.0:

```text
cargo fmt --check                                      PASS
cargo check                                           PASS
cargo test                                            PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo build --release                                 PASS
```

The previous validated baseline was commit `c5fbd78`. Commit `14b0a4f` supersedes it for Step 35 source. Steps 37 through 41 introduce source changes for the dry-run pack-load trust gate and related reporting. They require a new external validation run before they become a new authoritative validated baseline. Steps 42 and 43 are docs/spec/config-example only.


## Developer MVP alpha packaging

Status: closed for current Developer MVP baseline.

The primary alpha guide is `docs/DEVELOPER_MVP_ALPHA.md`. It documents what the current MVP can do, what it intentionally cannot do, safe configuration examples, the complete command walkthrough, troubleshooting, evidence/audit behavior, and known limitations.

Future decisions:

- whether to publish a shorter quickstart separate from the alpha guide
- whether to add generated example projects for common language ecosystems
- whether to add guided setup checks

## Provider expansion after OpenAI and Anthropic

Status: open.

Decision so far: mock, OpenAI, and Anthropic provider adapters are implemented behind the provider trait. Additional providers may be added later only if provider output remains untrusted and runtime policy/evidence/audit remain the authority boundary.

## Approval expiration format and enforcement

Status: closed for local MVP in Step 20.

Approval expiration uses strict UTC RFC3339 without fractional seconds:

```text
YYYY-MM-DDTHH:MM:SSZ
```

Approval request creation validates the optional expiration format. Approval recording rejects expired requests. Approval verification rejects expired or malformed expiration metadata, and approval-backed workflows rely on verification before use.

Future decisions:

- whether team or production modes require expiration on every approval
- whether to support fractional seconds or full RFC3339 offsets later

## Evidence failure after mutation

Status: open.

Patch apply prepares all file changes before writing, but final evidence creation still happens after writes. A future hardening step should define how to handle evidence write failure after a successful mutation.

Possible mitigation:

- pre-create an apply-attempt evidence directory before mutation
- append a pre-write audit event
- write final evidence after mutation
- define a recovery/audit marker if final evidence cannot be completed

## Test Runner execution hardening

Status: first implementation closed; future hardening remains open.

Step 17 implements an allowlisted Test Runner, not arbitrary shell:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The command string comes from `.gadgets/config.yaml`, not from provider output or free-form user input. Commands are launched without `sh -c`, shell composition syntax is rejected, stdout/stderr evidence is capped, and secret-like output lines are redacted.

Future hardening decisions:

- whether stricter modes should require a separate approval for test execution
- whether to add OS-level sandboxing or job isolation
- whether to support a richer argument parser without becoming shell-compatible
- whether to make redaction rules configurable; Step 25 centralized fixed best-effort rules but did not add configurability

## Redaction model

Status: partially closed.

Step 25 centralized best-effort redaction for current evidence-producing providers. Configurable redaction, full DLP, provider-safe evidence summarization, and deeper secret scanning remain future work.

## Pack trust and signing

Status: diagnostics implemented; dry-run runtime gate implemented at checkpoint level; hard-deny enforcement not implemented.

Step 26 defines the pack trust/signing design in `specs/PACK_TRUST_SIGNING_SPEC.md`. Step 27 adds non-enforcing `gadgets pack trust check [--project <path>] <pack>` diagnostics. Step 28 adds non-enforcing `gadgets pack trust roots [--project <path>]` trust-root diagnostics. Step 29 defines the evidence and audit contract for pack trust checks, trust-root loading, signature verification, and pack-load denials. Step 30 emits diagnostic evidence and audit for the trust check and trust-root inspection commands. Step 31 adds non-enforcing `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` policy preview diagnostics. Step 34 adds non-enforcing Ed25519 verification diagnostics to `gadgets pack trust signature`. Step 35 updates `gadgets pack trust preview` to consume those real signature diagnostics while remaining non-enforcing. Step 36 defines the enforcement states, dry-run gate, effective source classification, evidence/audit failure behavior, rollback behavior, and config shape. Step 37 adds the first runtime dry-run gate before implemented Developer Pack actions; it writes warning or would-deny evidence/audit but does not hard-deny pack loading. Step 38 adds a diagnostic `gate-preview` command that reports the configured gate outcome for an installed pack and operation without executing Gadgets. Step 39 adds a read-only `gate-history` command that filters prior trust-gate audit events for operator review. Step 40 adds a read-only `gate-status` command for current trust-gate configuration posture. Step 41 adds a read-only `gate-summary` command for trust-gate event counts and hard-deny review posture.

Locked decisions:

- Pack trust is eligibility to load/use a pack, not action authority.
- Signed packs cannot bypass policy, capabilities, tool allowlists, zones, approvals, evidence, or audit.
- Pack identity includes content hashes.
- Signed packs should use a deterministic content manifest and detached signature record.
- Recommended primitives are SHA-256 and Ed25519.
- Safe mode can allow explicit unsigned local development packs with audit warning.
- Team mode should start with dry-run deny for unsigned, unknown, expired, mismatched, or invalid non-built-in packs.
- Production mode should start with dry-run deny and later move to hard-deny only after explicit approval.
- Effective built-in trust applies only when both the pack manifest and all loaded Gadget manifests are built-in runtime assets.
- Built-in packs with project-local Gadget overrides are `project_local_mixed` and follow project-local trust rules.

Still open for implementation:

- external validation of the Step 37 dry-run-only gate, Step 38 gate-preview reporting, Step 39 gate-history reporting, Step 40 gate-status reporting, and Step 41 gate-summary reporting
- later hard-deny enforcement after dry-run review
- trust-root management commands
- pack signing tooling
- registry/install/update behavior

## Git branch and commit hardening

Status: branch creation and approved local commit baselines closed; remote hardening remains open.

Step 18a implements observe-only local Git status. Step 18b implements protected local branch creation through `gadgets git branch create [--project <path>] <branch-name>`. Step 18c implements approved local commit scaffolding through `gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]`.

Closed for Step 18b:

- protected branch config shape is `git.protected_branches`
- exact entries protect exact names, and entries ending in `/` protect branch prefixes
- branch names are validated before execution
- branch creation does not require patch approval when the runtime has validated the branch name and rejected protected branches
- branch creation does not checkout, switch, stage, commit, push, fetch, open PRs, or run shell

Closed for Step 18c:

- commits bind to verified patch approval scope and the exact `proposed.patch` hash
- commit staging is limited to files named by the approved patch artifact
- preexisting staged changes are rejected
- protected current branches and detached HEAD are rejected before staging

Future decisions:

- whether commits should also require a successful named test-run evidence reference
- whether stricter modes require approval for branch creation
- how optional PR creation should be configured and gated
- whether remote PR creation is ever enabled by default


## Remote PR behavior

Closed for Step 24:

- local PR body generation writes evidence only
- `gadgets git pr body` verifies a patch approval before generation
- optional test and commit run IDs are evidence references only
- guarded GitHub PR creation is implemented only when explicitly enabled in config
- generated config keeps remote PR creation disabled by default
- generated config keeps remote PR dry-run enabled by default
- dry-run mode writes evidence without reading the token or making the GitHub mutation call
- allowed base branches are configured through `git.remote_pr.allowed_base_branches`
- allowed head branch prefixes are configured through `git.remote_pr.allowed_head_prefixes`
- duplicate-open-PR behavior is configured through `git.remote_pr.duplicate_strategy`, currently `fail` or `reuse`
- no GitHub/GitLab behavior is enabled by default
- no push, pull, fetch, merge, rebase, checkout, or switch is performed

Still open:

- whether to add GitLab support later
- whether live remote PR validation should become a separate gated release check
- whether to allow draft PR creation as a separate config option
- whether to support fork-style head refs later
- whether remote PR creation should remain permanently manual-only

## Step 32 through Step 41 status

Closed by Steps 32-37: signature metadata diagnostics, Ed25519 verification diagnostics, signature-aware policy preview, enforcement-state design, dry-run gate design, effective source classification, and narrow dry-run-only pack-load gate implementation.

Still open for later steps:

- external validation of the Step 37 through Step 41 source checkpoint
- hard-deny enforcement after dry-run review
- key algorithm support beyond the current `ed25519` metadata contract
- signing tooling

## Closed in Step 33 - cryptographic verification byte contract

Closed for design:

- version 1 signature algorithm is Ed25519
- version 1 hash algorithm is SHA-256
- pack manifest hash uses raw `pack.yaml` bytes
- content manifest hash uses raw `pack.contents.yaml` bytes
- signature payload format is deterministic line-based `gadgets-pack-signature-v1`
- trust-root matching fields are publisher, key id, algorithm, allowed pack id, and expiration

Still open for implementation:

- external validation of the Step 37 through Step 41 source checkpoint
- signing tools
- later hard-deny enforcement after dry-run review
- trust-root mutation commands
- registry and pack install/update flows


## Step 34 closed decisions

Closed for Step 34: diagnostic Ed25519 verification.

Locked outcomes:

- `gadgets pack trust signature` performs real Ed25519 verification when signed metadata and matching trust-root public keys are available.
- Verification uses the deterministic `gadgets-pack-signature-v1` payload.
- Verification remains diagnostic only and does not affect pack loading.
- Signing tools, trust-root mutation, pack install/update, registry downloads, Team/Production enforcement, and Gadget execution changes remain deferred.

## Step 35 closed decisions

Closed for Step 35: pack trust policy preview consumes real signature diagnostics.

The preview command now reports signature metadata decision, signature presence, cryptographic verification performed/valid status, content-manifest validity, and expiration status. Safe Mode still allows local development packs with warnings. Team and Production previews allow only valid trusted signatures diagnostically. Pack trust enforcement remains deferred.


## Step 37 closed decisions

Closed for Step 37: narrow dry-run-only runtime pack-load trust gate.

Locked outcomes:

- `pack_trust` config is parsed with Safe `warn-only`, Team `dry-run-deny`, and Production `dry-run-deny` defaults.
- runtime `hard-deny` remains deferred and is treated as `dry-run-deny` in Step 37.
- Safe Mode `hard-deny` config is rejected.
- built-in trust requires both the pack manifest and all loaded Gadget manifests to be built-in.
- built-in pack plus project-local Gadget override is `project_local_mixed`.
- warning and dry-run denial outcomes write evidence and audit before the Gadget action continues.
- required evidence/audit failure fails closed for project-local and mixed-source runtime actions.

Still open:

- Step 37 through Step 41 external Rust validation.
- hard-deny enforcement after dry-run evidence review.
- richer reporting around pack-load trust metrics.


## Step 38 closed decisions

Closed for Step 38: pack-load trust gate preview reporting.

Locked outcomes:

- `gadgets pack trust gate-preview [--project <path>] [--operation <operation>] <pack>` reports the configured runtime gate outcome for an installed pack.
- The command loads all declared Gadget material by default, or operation-specific Developer Pack Gadget material when `--operation` is provided.
- The command writes diagnostic evidence and appends `pack.trust.gate.previewed` plus `evidence.created` audit events.
- The command does not execute Gadgets, hard-deny pack loading, mutate trust roots, install packs, download packs, or run provider tools.


## Step 39 closed decisions

Closed for Step 39: pack-load trust gate history reporting.

- `gadgets pack trust gate-history [--project <path>] [--limit <n>]` is read-only.
- The command filters prior gate events from the audit ledger.
- The command does not create evidence, append audit events, execute Gadgets, or hard-deny pack loading.
- Hard-deny enforcement remains deferred.

## Step 40 closed decisions

Closed for Step 40: pack trust gate status reporting.

Decision: add a read-only `gadgets pack trust gate-status [--project <path>]` command that reports current pack-trust configuration posture and hard-deny deferral without writing evidence, appending audit events, loading packs, verifying signatures, executing Gadgets, or enforcing hard-deny pack loading.

## Step 41 closed decisions

Closed for Step 41: pack trust gate summary reporting.

Decision: add a read-only `gadgets pack trust gate-summary [--project <path>]` command that reads local configuration and prior trust-gate audit events, reports event counts, and prints a review posture for future hard-deny discussions without writing evidence, appending audit events, loading packs, verifying signatures, executing Gadgets, or enforcing hard-deny pack loading.


## Step 42 closed decisions

Closed for Step 42: AI RMF alignment and governance profile design.

Decision: add a docs/spec-only alignment model that maps Gadgets controls to NIST AI RMF-style Govern, Map, Measure, and Manage functions. This is an engineering alignment and not a compliance claim.

Locked outcomes:

- Governance maps to policies, approvals, runtime modes, pack trust, evidence requirements, and audit records.
- Mapping maps to provider profiles, model inventory, packs, Gadgets, capabilities, zones, data exposure labels, and effective source classification.
- Measurement maps to evidence bundles, audit verification, policy outcomes, trust-gate counts, signature diagnostics, redaction metrics, and test results.
- Management maps to Safe Mode, approval gates, dry-run/hard-deny states, rollback guidance, incident events, provider disablement, and containment procedures.

Still open:

- runtime AI risk inventory commands
- runtime data exposure enforcement
- AI risk incident event emission
- AI risk reporting
- external validation of Steps 37 through 41 is now the immediate next action
- hard-deny enforcement after dry-run evidence review


## Step 43 closed decisions

Closed for Step 43: provider and model inventory design.

Decision: add a docs/spec-only provider/model inventory contract that complements existing `model_profiles`. The inventory records governance metadata such as provider status, review status, network posture, approved runtime modes, allowed data labels, denied data labels, retention notes, model profile purpose, allowed task kinds, allowed packs, and allowed Gadgets.

Locked outcomes:

- provider inventory does not replace provider profiles
- inventory must never store secret values or provider credentials
- environment variable names may be recorded as non-secret configuration shape
- live provider use should require explicit inventory and review before Team or Production use in future work
- future enforcement should start with read-only reporting, then warning-only or dry-run checks before any hard-deny behavior

Still open:

- runtime provider/model inventory report command
- runtime data exposure label enforcement
- provider disablement workflow
- AI risk incident emission
- external validation of Steps 37 through 41 is now the immediate next action
- hard-deny enforcement after dry-run evidence review
