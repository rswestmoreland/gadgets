# Gadgets Framework - License Metadata Update Checkpoint

Date: 2026-05-13

## Scope

This checkpoint updates project licensing and author metadata only. It does not add runtime behavior.

## License decision

Gadgets Framework is dual-licensed under MIT OR Apache-2.0.

## Author and copyright

```text
Richard S. Westmoreland
dev@rswestmore.land
Copyright 2026 Richard S. Westmoreland
```

## Files added

- `LICENSE.md`
- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `AUTHORS.md`
- `COPYRIGHT.md`
- `GADGETS_FRAMEWORK_LICENSE_UPDATE_CHECKPOINT_2026_05_13.md`
- `GADGETS_FRAMEWORK_CODEX_PROMPT_STEP21_LICENSE_VALIDATION_2026_05_13.md`

## Files removed

- `LICENSE_DECISION_PENDING.md`

## Files updated

- `Cargo.toml`
- `crates/gadgets-approval/Cargo.toml`
- `crates/gadgets-cli/Cargo.toml`
- `crates/gadgets-core/Cargo.toml`
- `crates/gadgets-evidence/Cargo.toml`
- `crates/gadgets-ledger/Cargo.toml`
- `crates/gadgets-policy/Cargo.toml`
- `crates/gadgets-provider/Cargo.toml`
- `crates/gadgets-tools/Cargo.toml`
- `README.md`
- `docs/DECISION_RECORD.md`
- `docs/OPEN_DECISIONS.md`
- `docs/ROADMAP.md`
- `FILE_MANIFEST.txt`

## Validation performed in this environment

- Confirmed no `old unlicensed metadata` string remains.
- Confirmed no `LICENSE_DECISION_PENDING.md` file remains.
- Confirmed Cargo package metadata uses `MIT OR Apache-2.0`.
- Confirmed author metadata uses `Richard S. Westmoreland <dev@rswestmore.land>`.
- Confirmed ASCII scan passed.
- Confirmed ZIP integrity check passed after packaging.

## Rust validation

Rust/Cargo validation was not run in this environment. Run externally:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
