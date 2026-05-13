# Gadgets Framework - Updated Plan and Progress After Step 33

Date: 2026-05-13

## Current status

Step 33 is complete as a documentation/specification checkpoint.

The project now has a locked byte-level design for real pack cryptographic signature verification. No cryptographic verification code or enforcement was added in this step.

## Progress summary

- Core safety spine: complete and previously validated.
- Local Developer MVP: implemented and previously validated through Step 21 baseline.
- Remote PR path: implemented and later hardened with dry-run and branch constraints; external validation still pending after later source changes.
- Pack trust diagnostics: implemented through non-enforcing trust checks, trust-root inspection, policy preview, and signature metadata diagnostics.
- Pack cryptographic verification: design finalized in Step 33; implementation remains pending.

## Completed recent pack trust steps

- [x] Step 26: Pack trust/signing design.
- [x] Step 27: Non-enforcing pack trust inspection scaffold.
- [x] Step 28: Non-mutating trust-root inspection scaffold.
- [x] Step 29: Pack trust evidence/audit design.
- [x] Step 30: Diagnostic evidence/audit emission for trust diagnostics.
- [x] Step 31: Non-enforcing pack trust policy preview.
- [x] Step 32: Non-cryptographic signature metadata verification scaffold.
- [x] Step 33: Cryptographic verification design finalization.

## Step 33 locked items

- [x] Ed25519 selected for version 1 signatures.
- [x] SHA-256 selected for version 1 hashes.
- [x] Raw-byte hash contract for `pack.yaml` locked.
- [x] Raw-byte hash contract for `pack.contents.yaml` locked.
- [x] Deterministic line-based signature payload v1 locked.
- [x] Content manifest verification rules locked.
- [x] Trust-root matching rules locked.
- [x] Denial mapping locked.
- [x] Evidence artifacts for real verification defined.
- [x] Audit events for real verification defined.
- [x] Enforcement rollout order defined.

## Still not implemented

- [ ] Real Ed25519 signature verification.
- [ ] Pack signing tools.
- [ ] Trust-root mutation tools.
- [ ] Pack trust enforcement during pack loading.
- [ ] Team Mode pack-load enforcement.
- [ ] Production Mode pack-load enforcement.
- [ ] Pack install/update behavior.
- [ ] Registry downloads.
- [ ] Git push/fetch/pull/merge/rebase.
- [ ] Linux admin behavior.
- [ ] Database/cloud/deployment behavior.
- [ ] Arbitrary shell.

## Validation status

The last confirmed full external Rust validation baseline remains commit `c5fbd78`.

External validation should be rerun before release tagging because later steps added Rust source changes:

- Step 24
- Step 25
- Step 27
- Step 28
- Step 30
- Step 31
- Step 32

Required validation flow when ready:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Proceed with Step 34 - Ed25519 verification diagnostics.

Step 34 should add real cryptographic verification to `gadgets pack trust signature` only. It should remain diagnostic and non-enforcing. It should not add signing tools, trust-root mutation, pack install/update, registry downloads, or Team/Production enforcement.
