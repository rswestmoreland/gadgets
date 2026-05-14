# Step 37 - Pack Load Trust Dry-Run Gate

Date: 2026-05-13

## Scope

Step 37 implements the first narrow pack-load trust gate. The gate is dry-run only. It evaluates the effective loaded pack material before runtime Gadget actions and records warning or would-deny outcomes, but it does not block pack loading based on trust decision alone.

Hard-deny enforcement remains deferred until dry-run evidence has been reviewed and explicitly approved.

## Implemented behavior

Step 37 adds runtime config support for:

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

Defaults are applied when `pack_trust` is absent:

- Safe Mode: `warn-only`
- Team Mode: `dry-run-deny`
- Production Mode: `dry-run-deny`

`hard-deny` can be represented for future configuration, but Step 37 treats it as `dry-run-deny` at runtime. Safe Mode `hard-deny` is rejected by config validation.

## Effective source classification

The gate uses effective loaded source classification:

- `builtin`: the pack manifest and every loaded Gadget manifest are built-in runtime assets.
- `project_local`: the pack manifest comes from project-local files.
- `project_local_mixed`: the pack manifest is built-in, but at least one loaded Gadget manifest comes from a project-local override.

Only fully built-in material bypasses dry-run evidence. Mixed material follows project-local trust rules.

## Runtime gate insertion points

The Step 37 gate is called before these Developer Pack runtime operations execute:

- `ask`
- `git.status`
- `git.branch.create`
- `git.commit.approved-patch`
- `git.pr.body`
- `git.pr.create`
- `test.run`
- `patch.apply`

The gate is intentionally not added to `pack list`, `pack show`, or `pack validate`, because those commands inspect manifests rather than executing a Gadget action.

## Decision behavior

For project-local or mixed-source material:

- Safe Mode records `pack.trust.warning` when signature coverage is not verified.
- Team Mode records `pack.trust.dry_run_denied` when signature coverage is not verified.
- Production Mode records `pack.trust.dry_run_denied` when signature coverage is not verified.
- Valid signed project-local packs may proceed without warning when signature diagnostics verify.
- Mixed-source material is not treated as covered by a pack signature, because project-local Gadget overrides are not built-in runtime material.

Step 37 continues execution after warning or dry-run denial unless required evidence or audit cannot be written.

## Evidence artifacts

Step 37 writes pack-load trust evidence for warning and dry-run denial outcomes:

```text
summary.md
bundle.yaml
pack_load_trust_decision.txt
pack_identity.yaml
effective_pack_sources.yaml
pack_manifest_hash.txt
pack_signature_summary.yaml
signature_policy_inputs.txt
trust_findings.txt
enforcement_mode.txt
requested_operation.txt
rollback_guidance.txt
```

Dry-run denial also writes:

```text
dry_run_denial.txt
```

The evidence must not contain private keys, signing seeds, API tokens, provider credentials, secret-bearing config values, or unredacted secret values.

## Audit events

Step 37 emits:

- `pack.trust.warning`
- `pack.trust.dry_run_denied`
- `evidence.created`

These are dry-run gate records, not hard enforcement records. Step 37 does not emit `pack.trust.denied` or `pack.load.denied`.

## Failure behavior

If the gate is active for project-local or mixed-source runtime actions and required evidence cannot be written, the runtime fails closed and does not continue to the Gadget action.

If the gate is active for project-local or mixed-source runtime actions and required audit cannot be appended, the runtime fails closed and does not continue to the Gadget action.

Fully built-in pack and Gadget material is not blocked by Step 37 trust gate evidence/audit failures because no project-local pack trust warning or dry-run denial is being recorded.

## Non-goals

Step 37 does not add:

- hard-deny runtime enforcement
- arbitrary shell
- generic root-shell behavior
- provider SDK tool-call bypass
- Linux admin mutation
- database behavior
- cloud behavior
- deployment behavior
- Git push, pull, fetch, merge, rebase, checkout, or switch
- broader remote PR behavior
- GitLab support
- pack install/update behavior
- registry downloads
- signing tools
- trust-root mutation

## Recommended next step

Step 38 should review Step 37 dry-run behavior and validation output. If accepted, the next narrow implementation can improve reporting and add tests around dry-run evidence/audit paths. Hard-deny should remain deferred until dry-run results are reviewed.
