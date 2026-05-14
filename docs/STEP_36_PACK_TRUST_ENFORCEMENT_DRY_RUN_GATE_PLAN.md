# Step 36 - Pack Trust Enforcement Design and Dry-Run Gate Plan

Date: 2026-05-13

Status: docs-first design checkpoint. No runtime enforcement code is added by this step.

## Purpose

Step 36 defines how pack trust diagnostics become a future pack-load gate without weakening the current Gadgets authority boundary.

The goal is to make the future enforcement path exact before code changes:

- define Safe, Team, and Production behavior
- define enforcement states
- define config switches and safe defaults
- define audit events and evidence artifacts
- define effective source handling for built-in and project-local pack material
- define failure behavior if evidence or audit cannot be written
- define rollback behavior
- define a migration path from diagnostics to dry-run deny to hard deny

Step 36 does not enforce pack loading. It is the design and acceptance baseline for a later narrow dry-run implementation.

## Review findings carried into Step 36

The Step 35 validated baseline already has the important diagnostic pieces:

- pack trust inspection
- trust-root inspection
- signature metadata diagnostics
- Ed25519 verification diagnostics
- signature-aware policy preview
- evidence and audit emission for diagnostic commands

The main gap before enforcement is not cryptography. The main gap is defining the effective pack material that the runtime is deciding to load. A built-in pack manifest can still be combined with project-local Gadget overrides. Future enforcement must evaluate the effective loaded pack material, not only `pack.yaml`.

## Authority boundary

Pack trust only decides whether a pack is eligible to load. It does not authorize actions.

Even a trusted pack must still pass:

- manifest validation
- capability checks
- tool allowlists
- zone boundaries
- runtime mode restrictions
- approval checks
- evidence requirements
- audit requirements
- provider-output redaction and containment rules

Provider output, provider SDK tool calling, prompts, and model handoff features must not bypass pack trust, policy, evidence, audit, approvals, or runtime mode restrictions.

## Enforcement states

Future pack-load trust gates should use these exact states:

| State | Meaning | Load behavior |
|---|---|---|
| `off` | No pack-load trust gate is active. | Allow loading after existing manifest validation. |
| `warn-only` | Evaluate trust and record warnings, but do not block loading. | Allow loading. |
| `dry-run-deny` | Evaluate trust and record that loading would be denied, but do not block loading. | Allow loading for migration only. |
| `hard-deny` | Evaluate trust and block loading when the decision denies. | Deny loading before Gadget execution. |

`dry-run-deny` is the migration bridge between diagnostics and hard enforcement. It must write evidence and audit so operators can see exactly what would break before hard-deny is enabled.

## Runtime mode defaults

Recommended defaults for the first enforcement-aware implementation:

| Runtime mode | Default enforcement | Built-in effective source | Project-local unsigned | Project-local valid signature | Invalid, mismatched, expired, or unknown signature |
|---|---|---|---|---|---|
| Safe | `warn-only` | allow | allow with warning when local unsigned packs are allowed | allow | allow with warning |
| Team | `dry-run-deny` | allow | would deny | allow | would deny |
| Production | `dry-run-deny` first, `hard-deny` later after review | allow only when effectively built-in | would deny | allow | would deny |

Production hard-deny is the target end state, but it should not be enabled until dry-run evidence has been reviewed and accepted.

## Effective source classification

Pack trust must classify the effective loaded material, not just the pack manifest.

Recommended source classes:

| Source class | Meaning | Trust treatment |
|---|---|---|
| `builtin` | Pack manifest and every loaded Gadget manifest come from built-in runtime assets. | Built-in trust path. |
| `project_local` | Pack manifest comes from `.gadgets/packs/<pack>/pack.yaml` or other project-local source. | Project-local trust path. |
| `project_local_mixed` | Pack manifest is built-in, but one or more loaded Gadget manifests are project-local overrides. | Project-local trust path. |
| `registry` | Future installed pack from a registry. | Signed non-built-in trust path. |
| `archive` | Future installed pack from an archive. | Signed non-built-in trust path. |

Rules:

- A pack is effectively `builtin` only when the pack manifest and all loaded Gadget manifests are built-in runtime assets.
- A project-local `pack.yaml` shadowing a built-in pack is `project_local`.
- A project-local Gadget override inside a built-in pack makes the effective source `project_local_mixed`.
- `project_local_mixed` must not inherit the built-in trust decision.
- Effective source evidence must identify which loaded manifests came from built-in assets and which came from project-local files.

## Built-in packs

Built-in packs are trusted as part of the runtime distribution for this phase.

Boundaries:

- Step 36 does not add runtime distribution signing.
- Step 36 does not verify the binary, package manager, or release artifact.
- Built-in trust applies only when the effective source is fully built-in.
- Built-in trust does not bypass policy, capability, tool, zone, approval, evidence, or audit checks.

## Project-local packs

Project-local packs are useful for development, testing, and private extensions, but they must not be treated as built-in.

Rules:

- Safe Mode may allow unsigned project-local packs with warnings.
- Team Mode should require valid trusted signatures, except for future explicit policy exceptions.
- Production Mode should require valid trusted signatures.
- A project-local pack with invalid signature metadata should be treated as invalid signed material, not as harmless unsigned material.
- Expired signatures and expired trust roots should deny in Team/Production dry-run and hard-deny modes.

## Unsigned local packs

Unsigned local packs are allowed only for local development behavior.

Recommended behavior:

- Safe Mode `warn-only`: allow and warn when configured to allow unsigned local packs.
- Safe Mode without unsigned-local allowance: warn now; future hard-deny behavior remains a separate decision.
- Team Mode `dry-run-deny`: record would-deny and allow only because dry-run is active.
- Production Mode `dry-run-deny`: record would-deny and allow only because dry-run is active.
- Production Mode `hard-deny`: deny before Gadget execution.

## Invalid signatures

Invalid signatures include:

- missing required signature fields
- unsupported version
- unsupported algorithm
- invalid timestamp shape
- expired signature
- signature created after expiration
- manifest hash mismatch
- content manifest hash mismatch
- file hash mismatch
- duplicate or unsafe content manifest path
- missing `pack.yaml` in signed contents
- listed `pack.signature.yaml` in signed contents
- unknown publisher
- unknown key id
- key algorithm mismatch
- pack id not allowed by trust root
- expired trust root
- invalid public key encoding
- invalid signature encoding
- Ed25519 verification failure

Team and Production should treat invalid signatures as deny outcomes. Safe Mode may allow with warning only for local development behavior.

## Config contract

The future config shape should be:

```yaml
pack_trust:
  enabled: true

  enforcement:
    safe: warn-only
    team: dry-run-deny
    production: dry-run-deny

  safe_mode:
    allow_unsigned_local_packs: true

  evidence:
    require_for_pack_load_decisions: true

  audit:
    require_for_pack_load_decisions: true
```

Allowed enforcement values:

```text
off
warn-only
dry-run-deny
hard-deny
```

Safe defaults:

- Generated config should document the shape, but existing projects without `pack_trust` should keep current behavior until the gate implementation is added.
- `hard-deny` should require explicit operator configuration in the first implementation.
- `off` should be treated as an emergency rollback setting and documented as unsafe for Team/Production.
- Trust roots remain read-only inputs during runtime action execution.

## Audit events

Future pack-load trust gates should use these audit event types:

```text
pack.trust.checked
pack.trust.allowed
pack.trust.warning
pack.trust.dry_run_denied
pack.trust.denied
pack.load.denied
pack.signature.checked
pack.signature.verified
pack.signature.failed
pack.signature.expired
trust.root.loaded
trust.root.missing
trust.root.rejected
trust.root.expired
evidence.created
```

Audit records should include safe identifiers only:

- pack id
- pack version
- source class
- manifest hash
- content hash
- publisher
- key id
- runtime mode
- enforcement state
- decision kind
- run id when present
- loaded manifest source summary
- human-readable findings

Audit records must not include:

- private keys
- signing seeds
- API tokens
- provider credentials
- secret-bearing config values
- full unredacted provider output

## Evidence artifacts

Future pack-load trust decisions should write these artifacts:

```text
summary.md
bundle.yaml
pack_load_trust_decision.txt
pack_identity.yaml
effective_pack_sources.yaml
pack_manifest_hash.txt
pack_contents_summary.txt
pack_signature_summary.yaml
signature_policy_inputs.txt
trust_root_summary.yaml
trust_findings.txt
enforcement_mode.txt
requested_operation.txt
rollback_guidance.txt
```

For dry-run denial, also write:

```text
dry_run_denial.txt
```

For hard denial, also write:

```text
pack_load_denial.txt
```

Evidence must not include private keys, signing seeds, API tokens, provider credentials, full secret-bearing configs, or unredacted secret values.

## Failure behavior

Pack trust enforcement must fail closed for runtime actions when required evidence or audit cannot be written.

Rules:

- If a pack trust gate is active and evidence cannot be written, do not execute project-local or mixed-source pack actions.
- If a pack trust gate is active and audit cannot be appended, do not execute project-local or mixed-source pack actions.
- Built-in pack loading for non-executing informational commands may continue only when no Gadget action will run.
- Runtime actions must not proceed when required pack-load trust evidence or audit fails.
- Provider output must not override evidence or audit failure behavior.

## Migration path

Recommended rollout:

1. Keep Step 35 diagnostics and signature-aware preview as the validated baseline.
2. Complete Step 36 docs-first design. This step.
3. Implement a narrow dry-run gate that evaluates pack-load trust but never blocks loading.
4. Emit `pack.trust.dry_run_denied` evidence and audit for would-deny outcomes.
5. Review dry-run evidence across Safe, Team, and Production sample projects.
6. Enable Team hard-deny only after review.
7. Enable Production hard-deny only after a separate explicit approval.
8. Add signing tools later, after enforcement semantics are stable.
9. Add trust-root mutation commands later, with separate approvals and audit.

## Rollback behavior

Rollback is explicit operator action, not automatic runtime behavior.

Rules:

- Do not automatically fall back from Production to Safe Mode.
- If hard-deny causes operational breakage, an operator may explicitly set enforcement to `warn-only` or `off`.
- Such rollback should be documented as a local configuration change.
- Trust roots must not be mutated by rollback.
- No registry downloads, pack updates, or signing tools are introduced by rollback.

## Test plan names only

No tests are implemented in Step 36. Future implementation should use names like:

```text
pack_trust_safe_warn_only_allows_unsigned_local
pack_trust_team_dry_run_denies_unsigned_local
pack_trust_production_dry_run_denies_unsigned_local
pack_trust_builtin_allowed_all_modes
pack_trust_project_override_not_treated_as_builtin
pack_trust_project_gadget_override_makes_effective_source_mixed
pack_trust_valid_signature_allows_team_preview
pack_trust_invalid_signature_dry_run_denies
pack_trust_expired_signature_dry_run_denies
pack_trust_expired_trust_root_dry_run_denies
pack_trust_missing_evidence_blocks_runtime_action
pack_trust_missing_audit_blocks_runtime_action
pack_trust_dry_run_denial_writes_evidence_and_audit
pack_trust_hard_denial_blocks_pack_load_when_enabled
provider_handoff_cannot_bypass_pack_trust_gate
```

## Non-goals

Step 36 does not add:

- arbitrary shell
- generic root-shell behavior
- provider SDK tool-call bypass
- Linux admin mutation
- database behavior
- cloud behavior
- deployment behavior
- Git push, pull, fetch, merge, rebase, checkout, or switch
- broader remote PR creation
- GitLab support
- pack install or pack update behavior
- registry downloads
- signing tools
- trust-root mutation
- runtime pack-load denial
- secret exposure to providers or evidence

## Acceptance checklist

- [x] Safe Mode behavior defined.
- [x] Team Mode behavior defined.
- [x] Production Mode behavior defined.
- [x] Enforcement states defined.
- [x] Migration path from diagnostics to enforcement defined.
- [x] Config switches and safe defaults defined.
- [x] Audit events for pack-load trust decisions defined.
- [x] Evidence artifacts for pack-load trust decisions defined.
- [x] Built-in pack treatment defined.
- [x] Project-local pack treatment defined.
- [x] Unsigned local pack treatment defined.
- [x] Invalid signature treatment defined.
- [x] Expired signature and expired trust-root treatment defined.
- [x] Evidence/audit failure behavior defined.
- [x] Rollback and Safe Mode behavior defined.
- [x] Test plan names listed only.
- [x] Runtime enforcement remains deferred.
