# Step 27 - Pack Trust Inspection Scaffold

Date: 2026-05-13

## Status

Complete at checkpoint/code level. Rust validation should be rerun after this code change.

## Goal

Add a non-enforcing inspection command before cryptographic signature verification or pack trust enforcement.

Implemented command:

```text
gadgets pack trust check [--project <path>] <pack>
```

## What the command reports

The command reports:

- pack name and version
- source path or built-in source label
- source kind: `builtin` or `project_local`
- diagnostic decision:
  - `trusted_builtin`
  - `allowed_unsigned_local`
  - `signed_metadata_unverified`
- whether enforcement is enabled; Step 27 always reports `false`
- manifest SHA-256
- whether `.gadgets/trust/trusted_publishers.yaml` exists
- optional `pack.contents.yaml` location, hash, and file count
- optional `pack.signature.yaml` location, hash, publisher, key id, algorithm, pack id, pack version, and timestamp metadata
- findings with `info`, `warning`, or `error` severity

## Safety boundaries

Step 27 does not:

- enforce signed-pack requirements
- verify Ed25519 signatures
- create signatures
- edit trust roots
- install packs
- update packs
- download registry content
- execute Gadgets
- call providers
- grant new capabilities
- change Safe/Team/Production runtime behavior

## Implementation notes

New module:

```text
crates/gadgets-cli/src/pack_trust.rs
```

Updated CLI path:

```text
crates/gadgets-cli/src/main.rs
```

New dependency in `crates/gadgets-cli/Cargo.toml`:

```text
sha2 = "0.10"
```

The first implementation keeps pack trust inspection in the CLI because it is a local diagnostic path, not an action-execution provider. It loads pack manifests through the existing manifest loader and then inspects optional project-local trust metadata files.

Built-in packs are reported as `trusted_builtin` because they are part of the runtime distribution. Project-local unsigned packs are reported as `allowed_unsigned_local`. Project-local packs with signature metadata are reported as `signed_metadata_unverified` because cryptographic verification is intentionally not implemented yet.

## Acceptance checklist

- [x] `gadgets pack trust check [--project <path>] <pack>` is added.
- [x] Built-in packs report as `trusted_builtin`.
- [x] Project-local unsigned packs report as `allowed_unsigned_local`.
- [x] Project-local packs with signature metadata report as `signed_metadata_unverified`.
- [x] Manifest SHA-256 is reported.
- [x] Optional `pack.contents.yaml` is inspected.
- [x] Optional `pack.signature.yaml` is inspected.
- [x] Local trust-root-file presence is reported.
- [x] Unsafe paths inside `pack.contents.yaml` produce findings.
- [x] No enforcement is added.
- [x] No signing tools are added.
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
