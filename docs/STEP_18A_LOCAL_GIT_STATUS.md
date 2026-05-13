# Step 18a - Local Git Status

Date: 2026-05-12

## Scope

Step 18a adds the first Git workflow slice: local observe-only Git status.

Implemented command:

```bash
gadgets git status [--project <path>]
```

The command runs one fixed runtime-selected command:

```bash
git status --short --branch --untracked-files=normal
```

It is launched directly with `std::process::Command` and does not run through `sh -c`.

## Safety boundary

Step 18a does not implement:

- branch creation
- commits
- staging files
- push, pull, fetch, merge, rebase, or remote operations
- pull request creation
- arbitrary Git arguments
- arbitrary shell
- provider-side tool execution
- patch application
- test execution
- Linux admin actions
- database, cloud, or deployment behavior

The model cannot supply the Git command. The runtime selects the fixed observe command.

## Policy behavior

The Git Gadget manifest now declares `git.status`.

The provider builds a structured action request with:

- capability: `git.status`
- tool: `git.status`
- zone: `local_repo`
- path: `.`
- resource: `working_tree`

The built-in policy engine checks the declared capability, tool allowlist, zone, and filesystem path boundary before executing the fixed command.

## Evidence artifacts

The status provider writes a separate evidence bundle containing:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `git_status.txt`
- `stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `branch.txt`
- `policy_decision.txt`
- `assumptions.txt`

Git stdout and stderr are capped and secret-like lines are redacted before evidence write.

## Audit events

The status provider appends audit events for:

- `git.status.requested`
- `policy.checked`
- `action.allowed` or `action.denied`
- `git.status.started`
- `git.status.completed` or `git.status.failed`
- `evidence.created`
- `run.completed`

## Known limits

This is an observe-only local Git slice. It requires a local `git` executable and a Git working tree. A non-Git directory or missing Git executable is reported as a failed run or provider error.

As of the Step 18a checkpoint, branch creation and commit creation remained deferred until approval and protected-branch semantics were locked. Step 18b later added protected local branch creation.
