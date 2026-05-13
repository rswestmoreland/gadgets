# Step 21 Checkpoint - Guarded Remote PR Creation

Date: 2026-05-13

## Summary

Step 21 implemented guarded remote PR creation for GitHub behind explicit local
configuration. This step intentionally skipped Rust validation per the requested
order, so external validation remains required.

## Implemented

- Added `gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]`.
- Added `git.remote_pr` config with default disabled state.
- Added config validation for remote PR provider, API base, token env, owner, repo, and default base branch.
- Added `RemotePrProviderConfig`, `GitRemotePrRequest`, `GitRemotePrReport`, and `run_git_remote_pr_create()`.
- Added `git.pr.create` policy context for verified remote PR creation.
- Added one fixed GitHub API call to create a pull request.
- Added evidence bundle artifacts for remote PR creation.
- Added audit events for remote PR creation.
- Updated README, docs, specs, roadmap, example config, generated config, Git manifest, and file manifest.

## Guardrails preserved

- No Git push.
- No Git fetch, pull, merge, or rebase.
- No checkout or switch.
- No arbitrary shell.
- No arbitrary Git arguments.
- No model-provider tool execution.
- No patch apply inside PR creation.
- No test execution inside PR creation.
- No Linux admin, database, cloud, or deployment behavior.
- Remote PR creation remains disabled by default.

## Validation performed in this environment

- ZIP integrity check should be run during packaging.
- YAML parse check should be run during packaging.
- ASCII scan should be run during packaging.
- Path-length scan should be run during packaging.

Cargo/Rust validation was not run in this environment.

## External validation still required

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Run the external Rust validation flow and make bounded fixes only. Do not add
new behavior until the Step 21 checkpoint compiles and passes validation.
