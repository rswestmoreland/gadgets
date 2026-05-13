# Step 2 - Core Types and Manifest Parsing

Date: 2026-05-12

## Status

Implemented as the first code-bearing checkpoint.

## Scope

This step turns the Phase 0 contract model into Rust data structures in `crates/gadgets-core`.

Implemented modules:

- `manifest.rs`
- `pack.rs`
- `capability.rs`
- `permission.rs`
- `zone.rs`
- `handoff.rs`
- `action.rs`
- `policy.rs`
- `evidence.rs`
- `audit.rs`
- `validation.rs`
- `error.rs`

## What this step does

- Parses Gadget manifests from YAML.
- Parses Pack manifests from YAML.
- Validates required schema versions.
- Rejects unknown permission levels through typed enum parsing.
- Rejects malformed capability names.
- Rejects manifests with no capabilities.
- Rejects mutating capabilities without explicit boundaries.
- Rejects release-level capabilities without approval rules.
- Adds initial unit tests for valid and invalid manifests.

## What this step does not do

- It does not execute tools.
- It does not call model providers.
- It does not read or write project files at runtime.
- It does not create `.gadgets/` state.
- It does not write audit ledgers or evidence bundles yet.
- It does not implement Linux administration actions.

## Important design choice

Capability validation is intentionally conservative. Capability names must be lowercase ASCII dot-separated names. Unknown or malformed capability names are rejected early so the runtime can remain default-deny.

## Next step

Implement `gadgets init` and local project state creation.

Target layout:

```text
.gadgets/
  config.yaml
  packs/
  gadgets/
  zones/
  runs/
  ledger/
  evidence/
  approvals/
```

The init command should be idempotent, use Safe Mode by default, and require no provider key.
