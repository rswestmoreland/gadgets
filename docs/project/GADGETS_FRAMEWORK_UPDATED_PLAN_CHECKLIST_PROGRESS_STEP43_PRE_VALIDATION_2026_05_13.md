# Gadgets Framework Updated Plan and Checklist - Step 43 Pre-Validation Review

Date: 2026-05-13

## Current checkpoint

```text
Step 43 pre-validation review and drift cleanup
```

## Current source and validation status

Last externally validated source baseline:

```text
Step 35 validated commit: 14b0a4f
```

Current checkpoint includes source changes from Steps 37 through 41 plus docs/spec changes from Steps 42 and 43. Steps 37 through 41 require external Rust validation before release-ready status can be claimed.

## Progress summary

| Area | Status | Notes |
|---|---:|---|
| Core runtime safety spine | Complete through Step 35 validation | Core types, init, ledger, evidence, policy, provider profiles, manifest loading, approval-bound patch flow, allowlisted tests, local Git, remote PR safety, redaction, and signature-aware pack trust preview were validated at commit `14b0a4f`. |
| Pack trust dry-run gate | Implemented, validation pending | Steps 37 through 41 added dry-run gate, preview, history, status, and summary reporting. |
| AI RMF governance alignment | Docs/spec complete | Step 42 added the governance profile design. |
| Provider/model inventory | Docs/spec complete | Step 43 added provider/model inventory design and future config shape. |
| External validation | Ready to run | This checkpoint prepares Codex validation. |

## Drift addressed

- Active roadmap/planning text now points to external validation as the immediate next action.
- Historical step notes remain preserved as historical records.
- A Codex validation prompt has been added under `docs/project/`.

## Required external validation

Run from the repository root:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Next action

Send the Codex prompt in `docs/project/GADGETS_FRAMEWORK_CODEX_PROMPT_STEP43_VALIDATION_2026_05_13.md` and review the validation output.

If validation fails, apply only bounded fixes required by the failing command and rerun the required flow from the earliest affected command.

## Not next

Do not proceed to Step 44 or hard-deny enforcement until validation results are reviewed.
