# Step 35 - Pack Trust Policy Preview With Signature Results

Date: 2026-05-13
Status: complete and externally validated in commit `14b0a4f`

## Purpose

Step 35 updates the non-enforcing `gadgets pack trust preview` diagnostic so it consumes the real signature diagnostic result added in Step 34.

Before this step, policy preview described future Safe, Team, and Production behavior without using Ed25519 verification results. After this step, preview decisions are based on the same signature metadata, content-manifest, trust-root, expiration, and Ed25519 verification checks used by `gadgets pack trust signature`.

The command remains diagnostic only. It does not enforce pack loading, mutate trust roots, install packs, download packs, execute Gadgets, or enable Team/Production pack-load gates.

## Command

```bash
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
```

## Implemented behavior

The preview now consumes signature diagnostics before computing the policy preview result.

For built-in packs:

- preview allows load in Safe, Team, and Production modes
- no project-local signature is required
- the pack remains trusted as part of the runtime distribution

For project-local packs in Safe Mode:

- preview allows local packs for developer workflows
- verified signatures are reported when present
- missing, invalid, expired, or mismatched signatures are reported as warnings
- no enforcement occurs

For project-local packs in Team Mode:

- preview allows load only when signature diagnostics report a valid trusted signature
- preview denies load when signature metadata is missing, invalid, expired, mismatched, or not linked to trusted publisher metadata
- the denial is diagnostic only

For project-local packs in Production Mode:

- preview allows load only when signature diagnostics report a valid trusted signature
- preview denies load when signature metadata is missing, invalid, expired, mismatched, or not linked to trusted publisher metadata
- the denial is diagnostic only

## Evidence

The preview evidence now includes signature policy inputs in addition to the existing policy preview artifacts.

Artifacts include:

```text
pack_trust_policy_preview.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_trust_decision.txt
signature_policy_inputs.txt
trust_findings.txt
policy_mode.txt
```

`pack_trust_policy_preview.txt` records:

- runtime mode
- preview decision
- whether load would be allowed
- whether a verified signature would be required
- whether a trust root would be required
- enforcement-active status
- signature metadata decision
- signature presence
- cryptographic verification performed flag
- cryptographic verification valid flag
- content manifest valid flag
- signature expiration status
- trust-root expiration status

`signature_policy_inputs.txt` records the signature-derived policy inputs separately so future enforcement work can compare preview behavior against eventual runtime gates.

## Audit

The command continues to append:

```text
pack.trust.policy.previewed
evidence.created
```

These audit events remain diagnostic and non-authoritative. They are not pack-load allow/deny events.

## Boundary

Step 35 does not add:

- pack trust enforcement
- signing tools
- trust-root mutation
- pack install/update commands
- registry downloads
- Team/Production pack-load enforcement
- Gadget execution behavior changes
- arbitrary shell
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior

## Acceptance checklist

- [x] `gadgets pack trust preview` consumes signature diagnostic results.
- [x] Built-in packs preview as trusted in all modes.
- [x] Safe Mode preview allows project-local packs with signature warnings when not verified.
- [x] Team Mode preview allows only valid trusted signatures diagnostically.
- [x] Production Mode preview allows only valid trusted signatures diagnostically.
- [x] Missing signatures produce deterministic preview denial in Team/Production modes.
- [x] Invalid, expired, or mismatched signatures produce deterministic preview denial in Team/Production modes.
- [x] Preview evidence includes signature policy inputs.
- [x] Preview audit remains diagnostic only.
- [x] Pack loading is not enforced.

## Validation note

External Rust validation was rerun after Step 35 and passed end-to-end in commit `14b0a4f`. The validation used Rust/Cargo 1.89.0 and passed:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

The validation fixes were bounded to formatting, compile correctness, test/lint correctness, and minimal bug fixes discovered by the validation flow. No new feature scope was added.
