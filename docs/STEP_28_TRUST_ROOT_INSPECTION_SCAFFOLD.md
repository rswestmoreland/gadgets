# Step 28 - Trust Root Inspection Scaffold

Date: 2026-05-13

## Status

Complete at checkpoint/code level. Rust validation should be rerun after this source change.

## Goal

Add a non-mutating diagnostic command for local pack trust roots before trust-root mutation, signature verification, or Team/Production enforcement.

Implemented command:

```text
gadgets pack trust roots [--project <path>]
```

## What the command reports

The command reports:

- trust-root file path
- whether `.gadgets/trust/trusted_publishers.yaml` exists
- whether the file parsed
- trust-root file version
- trusted publisher count
- publisher summaries:
  - publisher
  - key id
  - algorithm
  - whether a public key is present
  - allowed pack id count
  - expiration timestamp when present
- findings with `info`, `warning`, or `error` severity

## Safety boundaries

Step 28 does not:

- verify cryptographic signatures
- enforce signed-pack requirements
- create signatures
- edit trust roots
- add trust roots
- delete trust roots
- install packs
- update packs
- download registry content
- execute Gadgets
- call providers
- grant new capabilities
- change Safe/Team/Production runtime behavior

## Implementation notes

Updated module:

```text
crates/gadgets-cli/src/pack_trust.rs
```

Updated CLI path:

```text
crates/gadgets-cli/src/main.rs
```

The implementation reuses the Step 27 `pack_trust` module. It reads `.gadgets/trust/trusted_publishers.yaml` only when present, parses it as YAML, and reports structural metadata. It does not treat the file as authoritative for any runtime permission decision.

The expected trust-root shape remains documented in:

```text
specs/PACK_TRUST_SIGNING_SPEC.md
```

## Acceptance checklist

- [x] `gadgets pack trust roots [--project <path>]` is added.
- [x] Missing trust-root file reports as absent.
- [x] Present trust-root file reports parsed status.
- [x] Version is reported when available.
- [x] Publisher count is reported.
- [x] Publisher summaries are reported without printing raw private material.
- [x] Missing recommended publisher fields produce findings.
- [x] No enforcement is added.
- [x] No signing tools are added.
- [x] No trust-root mutation is added.
- [x] No registry download or pack install/update behavior is added.
- [x] No Gadget execution is added.

## Validation status

Rust validation was not rerun in this environment. External validation should be run before a release tag:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
