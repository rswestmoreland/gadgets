# Gadgets Framework Updated Plan Checklist and Progress - Step 38

Date: 2026-05-13

## Current checkpoint

Step 38 - Pack load trust gate preview reporting.

Source baseline: Step 37 dry-run gate checkpoint based on Step 35 externally validated commit `14b0a4f`.

Validation status: external Rust validation intentionally deferred until more work is complete.

## Progress estimate

| Scope | Current estimate | Notes |
|---|---:|---|
| Core safety spine through guarded remote PR creation | 100% | Previously validated at Step 35. |
| Local Developer MVP | 98-99% | Core workflow remains complete; Step 37 adds trust dry-run gating before runtime actions and Step 38 adds preview reporting. |
| Pack trust/signing path | 73-78% | Design, diagnostics, trust-root inspection, Ed25519 diagnostics, signature-aware preview, dry-run runtime gate, and gate-preview reporting are in place. Hard-deny, signing tools, and registry/install flows remain future work. |
| Full Gadgets Framework roadmap | 61-65% | Developer workflow is strong and pack trust has reached runtime dry-run gating plus operator preview reporting. Team workflows, Linux Admin packs, database/cloud/deployment packs, hard-deny, signing tools, AI RMF governance controls, and UI/team integrations remain future work. |

## Completed in Step 38

- [x] Added `gadgets pack trust gate-preview [--project <path>] [--operation <operation>] <pack>`.
- [x] Added a pure gate-decision helper shared by runtime dry-run gate logic and preview reporting.
- [x] Added operation-specific Developer Pack Gadget material selection for preview reporting.
- [x] Reports configured enforcement state.
- [x] Reports effective Step 37 enforcement state.
- [x] Reports hard-deny deferral.
- [x] Reports effective source kind.
- [x] Reports loaded Gadget manifest sources.
- [x] Reports whether signature coverage applies to the effective source.
- [x] Writes diagnostic evidence for the gate preview.
- [x] Appends `pack.trust.gate.previewed` and `evidence.created` audit records.
- [x] Added unit-test coverage names for the pure gate decision helper and Developer Pack operation mapping.
- [x] Updated active docs and specs.

## Not completed in Step 38

- [ ] External Rust validation.
- [ ] Hard-deny runtime pack-load enforcement.
- [ ] Signing tools.
- [ ] Trust-root mutation commands.
- [ ] Pack install/update behavior.
- [ ] Registry downloads.
- [ ] Linux admin mutation.
- [ ] Database, cloud, or deployment packs.

## Deferred validation checklist

When validation resumes, run:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Expected focus areas:

- pack trust gate-preview command parsing
- gate decision helper tests
- operation-specific Developer Pack Gadget selection
- evidence/audit emission for gate-preview
- no clippy warnings from helper reuse
- no formatting drift

## Recommended next step

Continue with another bounded non-validation step, or run external validation when ready. Hard-deny should remain deferred until dry-run behavior and evidence are reviewed and explicitly approved.
