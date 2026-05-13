# Step 27 Checkpoint - Pack Trust Inspection Scaffold

Date: 2026-05-13

## Summary

Step 27 adds a non-enforcing pack trust inspection command:

```text
gadgets pack trust check [--project <path>] <pack>
```

The command gives users visibility into pack trust metadata before the project implements signature verification or trust enforcement.

## Files changed

Code:

- `crates/gadgets-cli/Cargo.toml`
- `crates/gadgets-cli/src/main.rs`
- `crates/gadgets-cli/src/pack_trust.rs`

Docs/specs:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/DECISION_RECORD.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/OPEN_DECISIONS.md`
- `docs/ROADMAP.md`
- `docs/STEP_27_PACK_TRUST_INSPECTION_SCAFFOLD.md`
- `docs/STEP_27_PACK_TRUST_INSPECTION_SCAFFOLD_CHECKPOINT.md`
- `specs/PACK_TRUST_SIGNING_SPEC.md`
- `FILE_MANIFEST.txt`

Progress artifact:

- `GADGETS_FRAMEWORK_UPDATED_PLAN_CHECKLIST_PROGRESS_STEP27_2026_05_13.md`

## Behavior added

The trust check command reports:

- pack source and source kind
- diagnostic trust decision
- manifest SHA-256
- trust-root-file presence
- content manifest metadata when present
- signature metadata when present
- findings for missing metadata, unsafe content paths, and basic hash mismatches

## Explicit non-goals preserved

Step 27 does not add:

- cryptographic signature verification
- signature generation
- trust-root editing
- pack installation
- pack updates
- registry downloads
- Team/Production enforcement
- provider execution
- Gadget execution
- arbitrary shell
- Linux admin behavior
- database/cloud/deployment behavior

## Validation status

Cargo/Rust validation was not run in this environment after Step 27. Run external validation before release:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
