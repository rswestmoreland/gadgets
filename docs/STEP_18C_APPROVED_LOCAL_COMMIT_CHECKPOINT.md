# Step 18c Checkpoint - Approved Local Commit Scaffolding

Date: 2026-05-12

## Summary

Step 18c implemented approved local commit scaffolding for the Developer Pack.

The new command is:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

It creates one local commit only from files named by a verified approved patch request.

## Implemented

- Added CLI handling for `gadgets git commit approved-patch`.
- Added `crates/gadgets-tools/src/git_commit.rs`.
- Exported the Git commit provider from `gadgets-tools`.
- Added `approved_git_commit` policy context.
- Added deterministic policy handling for `git.commit.create` with verified approval context.
- Updated the Git Gadget manifest description and evidence requirements.
- Verified patch approval request and approval record before staging.
- Verified exact patch artifact hash and approval scope hash before staging.
- Extracted approved file paths from `proposed.patch`.
- Rejected detached HEAD.
- Rejected protected current branches.
- Rejected preexisting staged changes.
- Staged only approved patch files.
- Verified staged files are a subset of approved files before commit.
- Created one local commit through fixed Git command arguments.
- Attempted best-effort `git reset -- <approved-files>` if commit fails after staging.
- Captured Git stdout, stderr, exit status, duration, commit hash, branch, approved files, and staged files.
- Wrote separate evidence bundles.
- Appended audit events.
- Updated README, docs, specs, roadmap, and file manifest.

## Not implemented

- checkout or switch
- staging arbitrary files
- committing arbitrary staged files
- push, pull, fetch, merge, or rebase
- PR creation
- provider-side tool execution
- patch application inside Git commit provider
- test execution inside Git commit provider
- arbitrary shell
- Linux admin, database, cloud, deployment, or production behavior

## Validation status

Static packaging checks were run in the chat environment:

- ZIP integrity check
- ASCII scan
- path-length scan
- stale wording scan for Step 18c scope

Historical checkpoint note: Rust validation was not run at this step. Superseded by Step 22: full Rust validation passed at commit c5fbd78.

Historical checkpoint note: superseded by Step 22 validation passed at commit c5fbd78. Original validation commands were:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Step 19 should start with PR body generation only, then optional PR creation behind explicit configuration. Remote behavior should remain disabled by default.
