# Step 25 checkpoint - Secret/output redaction hardening

Date: 2026-05-13

## Checkpoint summary

Step 25 centralized the existing best-effort output redaction behavior into a shared helper and applied it to current evidence-producing providers.

## Code changes

Added:

- `crates/gadgets-tools/src/redaction.rs`

Updated:

- `crates/gadgets-tools/src/lib.rs`
- `crates/gadgets-tools/src/test_runner.rs`
- `crates/gadgets-tools/src/git_status.rs`
- `crates/gadgets-tools/src/git_branch.rs`
- `crates/gadgets-tools/src/git_commit.rs`
- `crates/gadgets-tools/src/pr_body.rs`
- `crates/gadgets-tools/src/remote_pr.rs`

## Behavior changes

- Test Runner output evidence now uses the shared redaction helper.
- Git command output evidence now uses the shared redaction helper.
- Local PR body text and referenced evidence summaries now use the shared redaction helper.
- Remote PR API response evidence now uses the shared redaction helper.
- Output truncation remains capped and UTF-8-safe.

## Non-goals preserved

- No arbitrary shell.
- No root-shell Gadget.
- No Git push/fetch/pull/merge/rebase/checkout/switch.
- No Linux admin behavior.
- No database/cloud/deployment behavior.
- No provider-side tool execution bypass.

## Validation performed in this environment

- ZIP integrity check.
- ASCII scan.
- YAML parse scan.
- Path-length scan.
- Build-artifact scan.

External Rust validation was not run, per user request to hold validation until more work is complete.
