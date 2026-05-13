# Step 19 - Local PR Body Generation

Date: 2026-05-12

## Goal

Step 19 adds local pull request body generation for the Developer Pack.

This is not remote PR creation. It writes reviewable Markdown evidence that a human or a later explicitly configured integration can use when creating a pull request.

## Command

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

## Safety boundary

The PR body generator:

- verifies the approval request, approval record, and expiration
- verifies the exact approved patch artifact through the approval model
- summarizes approved patch files, additions, and deletions
- optionally references a prior test evidence bundle
- optionally references a prior commit evidence bundle
- writes `pr_body.md` and related artifacts as evidence
- appends audit events

It does not:

- create a remote PR
- call GitHub, GitLab, or another remote Git provider API
- push, pull, fetch, merge, or rebase
- checkout or switch branches
- stage files
- create commits
- apply patches
- run tests
- call a model provider
- execute shell commands
- perform Linux admin, database, cloud, deployment, or production actions

## Policy

The Git Gadget manifest declares `git.pr.body.generate` and allows the `git.pr.body.generate` tool.

The runtime evaluates the action before evidence is written. The action is local evidence generation only, so it does not require the remote `git.pr.create` capability.

## Evidence artifacts

The PR body run writes a separate evidence bundle containing:

- `summary.md`
- `bundle.yaml`
- `pr_title.txt`
- `pr_body.md`
- `approval_verification.txt`
- `patch_summary.txt`
- `test_evidence.txt`
- `commit_evidence.txt`
- `policy_decision.txt`
- `assumptions.txt`

## Audit events

Expected audit events include:

- `git.pr.body.requested`
- `policy.checked`
- `action.allowed` or `action.denied`
- `git.pr.body.generated`
- `evidence.created`
- `run.completed`

## Acceptance checklist

- [x] `gadgets git pr body` command exists.
- [x] Approval request, approval record, and expiration are verified before generation.
- [x] PR body is generated as local evidence only.
- [x] Optional test evidence run ID can be referenced.
- [x] Optional commit evidence run ID can be referenced.
- [x] Remote PR creation was not implemented in Step 19; Step 21 later added guarded remote PR creation behind explicit config.
- [x] GitHub/GitLab API calls were not implemented in Step 19; Step 21 later added GitHub PR creation behind explicit config.
- [x] Push/fetch/merge/rebase behavior is not implemented.
- [x] Patch apply, test execution, and Git commit behavior are not invoked by PR body generation.
- [x] Evidence and audit are written.

## External validation

Cargo validation was not run in this environment. Run this externally when a Rust toolchain is available:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Step 20 update

Local PR body generation now inherits strict approval expiration enforcement through `verify_approval()`. Expired approvals are rejected before PR body evidence is generated.
