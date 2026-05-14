# Step 40 - Pack Trust Gate Status Reporting

Date: 2026-05-13

## Goal

Step 40 adds a read-only status report for the configured pack-load trust gate.

The new command gives operators a quick view of the current trust-gate posture before they run a Gadget action. It reports the active runtime mode, whether pack trust is enabled, the configured and effective enforcement state for Safe, Team, and Production modes, hard-deny deferral, evidence/audit requirements, installed packs, and the local audit ledger path.

This step improves operator reviewability without adding hard-deny enforcement or expanding runtime authority.

## Command

```bash
gadgets pack trust gate-status [--project <path>]
```

Default behavior:

- project path defaults to `.`
- reads `.gadgets/config.yaml`
- prints the configured pack-trust posture
- shows Step 37 effective enforcement for each runtime mode
- shows whether configured hard-deny is currently deferred to dry-run-deny
- does not load pack manifests
- does not verify signatures
- does not write evidence
- does not append audit events
- does not execute Gadgets

## Output shape

The command prints:

```text
Pack-load trust gate status
Project: <path>
Runtime mode: <safe|team|production>
Pack trust enabled: <true|false>

Mode enforcement:
  - safe: configured=<state>, effective_step37=<state>, hard_deny_deferred=<true|false>
  - team: configured=<state>, effective_step37=<state>, hard_deny_deferred=<true|false>
  - production: configured=<state>, effective_step37=<state>, hard_deny_deferred=<true|false>

Safe Mode allows unsigned local packs: <true|false>
Evidence required for pack-load trust decisions: <true|false>
Audit required for pack-load trust decisions: <true|false>
Installed packs: <list>
Ledger: <path>
```

## Relationship to Step 38 and Step 39

Step 38 previews a trust-gate decision for a specific installed pack and optional Developer Pack operation. That command writes diagnostic evidence and audit.

Step 39 reads prior trust-gate audit events and prints a focused event history.

Step 40 is lighter weight than both. It only reports the current configuration posture and does not evaluate an individual pack, write evidence, or append audit.

## Boundary

Step 40 does not add:

- hard-deny pack-load enforcement
- signing tools
- signature verification changes
- trust-root mutation
- pack install or update behavior
- registry downloads
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior
- broader Git behavior
- provider-side action bypass

## Validation status

External Rust validation remains deferred by user request. This checkpoint should be validated later with:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
