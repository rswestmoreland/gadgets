# Gadgets Framework - Updated Plan and Progress After Step 27

Date: 2026-05-13

## Current status

Step 27 is complete at checkpoint/code level.

The project now includes a non-enforcing pack trust inspection scaffold:

```text
gadgets pack trust check [--project <path>] <pack>
```

This gives visibility into pack source and trust metadata before signature verification and trust enforcement are implemented.

## Progress summary

| Scope | Progress | Notes |
|---|---:|---|
| Core safety spine | 100% | Implemented and previously validated. |
| Local Developer MVP | 95% | Implemented, alpha-packaged, and previously validated through Step 21. |
| Guarded remote PR MVP | 80-85% | Remote PR creation is guarded, dry-run by default, and safety-hardened; live provider-specific validation remains future work. |
| Pack trust/signing track | 25-30% | Design is locked and non-enforcing inspection is scaffolded; signature verification and enforcement remain future work. |
| Full Gadgets Framework roadmap | 47-51% | Developer workflow is strong; Team workflows, Linux admin packs, database/cloud/deployment packs, pack trust enforcement, and broader integrations remain future work. |

## Completed in Step 27

- [x] Added `gadgets pack trust check [--project <path>] <pack>`.
- [x] Added `crates/gadgets-cli/src/pack_trust.rs`.
- [x] Added `sha2` dependency to `gadgets-cli` for trust diagnostic hashes.
- [x] Reports built-in versus project-local pack source.
- [x] Reports `trusted_builtin`, `allowed_unsigned_local`, or `signed_metadata_unverified` diagnostic decisions.
- [x] Reports manifest SHA-256.
- [x] Inspects optional `pack.contents.yaml`.
- [x] Inspects optional `pack.signature.yaml`.
- [x] Reports local trust-root-file presence.
- [x] Adds findings for missing metadata, unsafe content paths, and basic hash mismatches.
- [x] Updates README, roadmap, implementation plan, architecture, decision record, open decisions, and pack trust spec.

## Explicitly not implemented

- [ ] Cryptographic signature verification.
- [ ] Signature generation.
- [ ] Trust-root editing.
- [ ] Pack install/update commands.
- [ ] Registry downloads.
- [ ] Team/Production pack trust enforcement.
- [ ] Safe Mode pack trust warnings during execution.
- [ ] Provider-side tool execution.
- [ ] Arbitrary shell.
- [ ] Linux admin behavior.
- [ ] Database/cloud/deployment behavior.

## Validation status

Last full external validation baseline remains the post-Step-21 validation at commit `c5fbd78`.

Steps 24, 25, and 27 include Rust source changes after that validation. External validation should be rerun before release:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Proceed with Step 28 as a docs-first or scaffold-only trust-root inspection step, or pause for external Rust validation if the next milestone will be release-oriented.

Suggested Step 28 if continuing feature work before validation:

```text
Step 28 - Trust root inspection scaffold
```

Possible command:

```text
gadgets pack trust roots [--project <path>]
```

Scope should remain non-mutating: list configured trust-root metadata only, without adding trust-root editing, signature verification, signing tools, registry downloads, or enforcement.
