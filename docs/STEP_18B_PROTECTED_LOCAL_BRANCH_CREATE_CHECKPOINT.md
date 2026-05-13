# Step 18b Checkpoint - Protected Local Branch Creation

Date: 2026-05-12

## Checkpoint summary

Implemented protected local Git branch creation as a narrow fixed-command provider.

New command:

```bash
gadgets git branch create [--project <path>] <branch-name>
```

## Implemented

- Added `crates/gadgets-tools/src/git_branch.rs`.
- Added `run_git_branch_create()` provider entrypoint.
- Added `GitBranchCreateRequest` and `GitBranchCreateReport`.
- Added CLI handling for `gadgets git branch create`.
- Added config support for `git.protected_branches`.
- Added default protected branches to generated `.gadgets/config.yaml`.
- Added protected branch config to the example project config.
- Added policy context support for validated local branch creation.
- Updated the Git Gadget manifest evidence requirements and description.
- Updated README, docs, specs, roadmap, and file manifest.

## Safety boundary

The provider runs only:

```bash
git branch <validated-branch-name>
```

It does not checkout or switch branches, stage files, commit, push, pull, fetch, merge, rebase, create PRs, run provider-side tools, apply patches, run tests, execute shell, or perform Linux admin, database, cloud, or deployment actions.

## Evidence

Each branch create run writes a separate evidence bundle with the fixed command, branch name, protected branch config, stdout, stderr, exit status, duration, policy decision, assumptions, and summary.

## Validation status

Performed in this environment:

- ASCII scan: passed
- path-length scan: passed
- changed YAML parse check: passed
- stale wording scan for current Step 18b scope: passed
- ZIP integrity check: passed at final package step

Not performed in this environment because Cargo/rustc/rustfmt are unavailable:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Still not implemented

- Git checkout or branch switching
- Git staging
- Git commit creation
- Git push, pull, fetch, merge, or rebase
- Pull request creation
- Arbitrary shell
- Provider-side tool execution
- Linux admin behavior
- Database, cloud, or deployment behavior

## Recommended next step

Step 18c was completed after this checkpoint as approved local commit scaffolding. Remaining Git work should focus on PR body generation and optional remote PR creation behind explicit configuration.
