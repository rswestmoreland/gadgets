# Step 25 - Secret/output redaction hardening

Date: 2026-05-13

## Goal

Centralize and improve best-effort redaction for evidence outputs before adding more integrations.

Step 25 does not add new execution authority. It keeps the current Developer MVP behavior intact while replacing duplicated per-provider redaction helpers with a shared helper in `gadgets-tools`.

## Scope

Implemented in this step:

- added a shared redaction helper module in `crates/gadgets-tools/src/redaction.rs`
- centralized best-effort line redaction and UTF-8-safe truncation
- applied shared redaction to Test Runner stdout/stderr evidence
- applied shared redaction to Git status output evidence
- applied shared redaction to Git branch creation output evidence
- applied shared redaction to Git commit output evidence
- applied shared redaction to local PR body text and evidence summaries
- applied shared redaction to remote PR API response evidence
- added unit tests for common secret-like lines and truncation behavior
- documented that this is not a complete DLP or secret-scanning system

## Redaction behavior

The helper redacts entire lines that contain common secret-like indicators, including examples such as:

- `.env`
- `password`
- `passwd`
- `secret`
- `token`
- `authorization`
- `bearer `
- `credential`
- `api_key`
- `apikey`
- `access_key`
- `secret_key`
- `private_key`
- `client_secret`
- `refresh_token`
- `id_token`
- `x-api-key`
- `.pem`
- `.p12`
- `.pfx`
- `.key`
- common GitHub and Slack token prefixes

The helper also preserves UTF-8 boundaries while truncating capped outputs.

## Evidence surfaces covered

Step 25 covers these evidence-producing surfaces:

- Test Runner stdout/stderr
- Git status stdout/stderr
- Git branch stdout/stderr
- Git commit stdout/stderr
- local PR body generated Markdown
- local PR body referenced evidence summaries
- remote PR API response evidence

## Safety notes

This is a best-effort local evidence safety measure. It is intentionally not described as full secret scanning, DLP, or a guarantee that sensitive material cannot appear in evidence.

Known limits:

- binary data is not deeply inspected
- structured secrets inside large JSON blobs may be redacted at line granularity only
- false positives are expected for some secret-like terms
- false negatives remain possible
- providers should still not receive raw evidence outputs unless a later provider-safe summary path is implemented

## Preserved non-goals

Step 25 does not add:

- arbitrary shell
- generic root-shell behavior
- provider-side tool execution
- Git push, fetch, pull, merge, rebase, checkout, or switch
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior
- pack signing
- production mode expansion

## Validation status

External Rust validation was intentionally deferred by user request after Step 24 and remains deferred after Step 25.

Recommended validation command sequence when ready:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
