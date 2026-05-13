# Step 18c - Approved Local Commit Scaffolding

Date: 2026-05-12

## Goal

Step 18c adds a narrow local Git commit slice for the Developer Pack.

The command is:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

This is not a general Git command runner. It can only create one local commit from files named by a verified approved patch request.

## Safety boundary

The provider verifies the approval request, approval record, and expiration before any Git staging or commit command. It verifies the exact `proposed.patch` SHA-256 and deterministic scope hash, extracts approved file paths from that patch, and uses only those files for staging and commit.

Before staging, the provider rejects:

- invalid project roots
- invalid commit messages
- invalid approved patch paths
- missing, expired, or invalid approval records
- detached HEAD
- protected current branches from `.gadgets/config.yaml` `git.protected_branches`
- preexisting staged changes

After staging, the provider verifies that every staged file is in the approved file set.

## Fixed Git commands

The provider may run these fixed local commands only:

```text
git rev-parse --abbrev-ref HEAD
git diff --cached --name-only --
git add -- <approved-files>
git commit -m <validated-message> -- <staged-approved-files>
git rev-parse HEAD
```

If commit creation fails after staging, it attempts a best-effort cleanup:

```text
git reset -- <approved-files>
```

All commands use `std::process::Command`. The provider does not use `sh -c`.

## Explicitly not implemented

Step 18c does not implement:

- arbitrary Git arguments
- checkout or switch
- push
- pull
- fetch
- merge
- rebase
- PR creation
- provider-side tool execution
- patch application
- test execution
- arbitrary shell
- Linux admin actions
- database, cloud, deployment, or production behavior

## Evidence artifacts

The commit provider writes a separate evidence bundle with:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `approval_verification.txt`
- `approved_files.txt`
- `staged_files.txt`
- `current_branch.txt`
- `commit_message.txt`
- `commit_hash.txt`
- `git_add_stdout.txt`
- `git_add_stderr.txt`
- `git_commit_stdout.txt`
- `git_commit_stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `policy_decision.txt`
- `assumptions.txt`

Git output is capped and secret-like lines are redacted before evidence write.

## Audit events

Expected audit events include:

- `git.commit.requested`
- `approval.used`
- `policy.checked`
- `action.allowed` or `action.denied`
- `git.commit.stage_started`
- `git.commit.started`
- `git.commit.completed` or `git.commit.failed`
- `evidence.created`
- `run.completed`

## Relationship to patch apply and tests

Patch apply remains separate. The commit provider does not apply patches.

Test running remains separate. The commit provider does not run tests.

Step 18c binds commit behavior to patch approval evidence only. A future hardening step may optionally require successful named test-run evidence before commit.

## Step 20 update

Approved local commits now inherit strict approval expiration enforcement through `verify_approval()`. Expired approvals are rejected before staging can begin.
