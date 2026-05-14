# Step 38 - Pack Load Trust Gate Preview Reporting

Date: 2026-05-13

## Scope

Step 38 adds read-only reporting for the Step 37 pack-load trust dry-run gate.

The new command lets an operator preview the configured runtime gate decision for an installed pack before a Gadget action is executed. It is intended to make Step 37 dry-run behavior easier to inspect across Safe, Team, and Production modes without moving to hard-deny enforcement.

## Implemented command

```text
gadgets pack trust gate-preview [--project <path>] [--operation <operation>] <pack>
```

The command:

- loads `.gadgets/config.yaml`
- uses the configured runtime mode
- requires the pack to be listed in `installed_packs`
- loads the effective pack manifest
- loads either all declared Gadget manifests or the Gadget manifests used by the selected Developer Pack operation
- classifies effective source material as `builtin`, `project_local`, or `project_local_mixed`
- runs the existing signature-aware pack trust policy preview
- reports the Step 37 gate action and decision
- writes diagnostic evidence
- appends diagnostic audit events

## Supported operation names

`--operation all` is the default and loads all declared Gadget manifests for the pack.

For the built-in Developer Pack, operation-specific previews are supported for:

```text
ask
git.status
git.branch.create
git.commit.approved-patch
git.pr.body
git.pr.create
test.run
patch.apply
```

Operation-specific previews for non-Developer packs are intentionally deferred. Use `--operation all` for those packs.

## Gate actions reported

The command reports one of these gate actions:

```text
disabled
off
builtin-bypass
verified-signature
warn-only
dry-run-deny
```

The command also reports:

- configured enforcement state
- effective Step 37 enforcement state
- whether `hard-deny` is deferred
- whether the decision would continue in Step 37
- whether evidence is required for runtime pack-load decisions
- whether audit is required for runtime pack-load decisions
- whether the signature covers the effective source material
- loaded Gadget manifest sources

## Evidence and audit

The command writes pack-load trust preview evidence using the existing pack-load trust evidence shape. It also appends:

```text
pack.trust.gate.previewed
evidence.created
```

These events are diagnostic records. They are not runtime warning, dry-run denial, or hard-denial events.

## Non-goals

Step 38 does not add:

- hard-deny runtime enforcement
- signing tools
- trust-root mutation
- pack install/update behavior
- registry downloads
- arbitrary shell
- generic root-shell behavior
- provider SDK tool-call bypass
- Linux admin mutation
- database behavior
- cloud behavior
- deployment behavior
- broader Git behavior
- GitLab support

## Test coverage added

Step 38 adds unit-test coverage for the pure gate decision helper and Developer Pack operation-to-Gadget mapping.

Test plan names:

```text
safe_mode_project_local_unsigned_warns
team_mode_project_local_unsigned_dry_run_denies
production_hard_deny_remains_deferred_to_dry_run
builtin_effective_source_bypasses_recording
verified_project_local_signature_bypasses_recording
developer_operation_maps_to_expected_gadgets
```

## Validation note

External Rust validation is intentionally deferred until more work is complete. The next external validation should include:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
