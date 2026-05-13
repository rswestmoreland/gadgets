# Gadgets Framework - Step 11 Pack Validate Checkpoint Overview v0.1

Date: 2026-05-12

## Completed step

Step 11 implements `gadgets pack validate` before live provider integration.

## New command

```bash
gadgets pack validate [--project <path>] [--strict] [pack]
```

Behavior:

- With no pack name, validates all packs listed in `.gadgets/config.yaml` under `installed_packs`.
- With a pack name, validates that named pack from project-local or built-in sources.
- Loads project-local pack manifests before built-in pack manifests.
- Loads project-local Gadget manifests before built-in Gadget manifests.
- Reports declared Gadget status as valid, missing, or invalid.
- Treats missing Gadget manifests as warnings by default.
- Treats missing Gadget manifests as errors with `--strict`.
- Exits non-zero when validation errors are present.

## Added built-in Developer Pack manifests

The Developer Pack now has built-in manifest files for all declared Gadgets:

- coordinator
- policy
- audit.ledger
- approval
- filesystem.read
- patch.writer
- test.runner
- git.pr
- documentation.writer
- secrets.guardian

At Step 11, only `filesystem.read` was executable. Current status has advanced: `patch.writer` now supports plan-only patch evidence and approved local patch application. The remaining Developer Pack manifests are still contract placeholders.

## Validation checks

Pack validation now checks:

- pack manifest load/parse success
- declared Gadget manifest load/parse success when available
- loaded Gadget `metadata.name` matches the pack declaration
- Gadget permission level does not exceed pack `safety.highest_permission_level`, when declared
- missing Gadget manifests are warnings or strict errors

The existing core manifest parser still enforces malformed capability names, invalid permission levels, mutating capabilities without explicit boundaries, and release capabilities without approval rules.

## Updated files and docs

Updated or added:

- `crates/gadgets-cli/src/manifest_loader.rs`
- `crates/gadgets-cli/src/main.rs`
- `packs/developer/gadgets/*.yaml`
- `docs/STEP_11_PACK_VALIDATE.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `specs/PACK_MODEL.md`
- `README.md`
- `FILE_MANIFEST.txt`

## Still not implemented

- arbitrary shell execution
- filesystem writes
- patch application
- allowlisted test execution was not part of this historical Step 11 checkpoint
- Git/PR behavior
- Linux admin actions
- database actions
- cloud actions

## Validation performed in this environment

- ZIP integrity check passed.
- ASCII scan passed.
- Path-length scan passed.
- YAML parse sanity check passed for built-in pack and Gadget manifests.
- Developer Pack declared Gadget manifest presence check passed.

Rust compilation and unit tests were not run because Cargo/Rust is not installed in this sandbox.

## Recommended next step

Implement the first live provider adapter, likely OpenAI, behind the existing provider trait.

The provider must remain non-authoritative:

- it can produce structured Coordinator output
- it can request handoffs
- it cannot execute tools directly
- the runtime still validates handoffs
- the runtime still enforces policy
- the runtime still produces evidence
- the runtime still appends audit events
