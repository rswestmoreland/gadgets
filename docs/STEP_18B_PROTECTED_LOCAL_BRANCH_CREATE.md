# Step 18b - Protected Local Branch Creation

Date: 2026-05-12

## Scope

Step 18b adds the first local Git mutation slice: protected local branch creation.

Implemented command:

```bash
gadgets git branch create [--project <path>] <branch-name>
```

The command runs one fixed runtime-selected command:

```bash
git branch <validated-branch-name>
```

It is launched directly with `std::process::Command` and does not run through `sh -c`.

## Safety boundary

Step 18b does not implement:

- checkout or branch switching
- staging files
- commits
- push, pull, fetch, merge, rebase, or remote operations
- pull request creation
- arbitrary Git arguments
- arbitrary shell
- provider-side tool execution
- patch application
- test execution
- Linux admin actions
- database, cloud, or deployment behavior

The model cannot supply the Git command. The runtime selects the fixed branch creation command and passes the validated branch name as a single process argument.

## Protected branch config

Generated project config now includes:

```yaml
git:
  protected_branches:
    - main
    - master
    - trunk
    - production
    - prod
    - release/
```

Exact entries such as `main` protect that exact branch name. Entries ending in `/`, such as `release/`, protect branch names below that prefix, such as `release/1.0`.

Protected branch names are rejected before the Git command is executed.

## Branch name validation

The runtime rejects branch names that are empty, too long, start with `-`, contain parent-ref patterns, contain `@{`, contain empty path segments, end with `.lock`, are `HEAD`, or contain characters outside the narrow ASCII branch-name set.

This prevents option injection and avoids shell-style interpretation because no shell is used.

## Policy behavior

The Git Gadget manifest declares `git.branch.create`.

The provider builds a structured action request with:

- capability: `git.branch.create`
- tool: `git.branch.create`
- zone: `local_repo`
- path: `.`
- resource: `branch:<branch-name>`

The built-in policy engine checks the declared capability, tool allowlist, zone, and filesystem path boundary before executing the fixed command. The policy context allows this narrow local branch creation only after runtime branch-name validation and protected-branch rejection.

## Evidence artifacts

The branch provider writes a separate evidence bundle containing:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `branch_name.txt`
- `protected_branches.txt`
- `stdout.txt`
- `stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `policy_decision.txt`
- `assumptions.txt`

Git stdout and stderr are capped and secret-like lines are redacted before evidence write.

## Audit events

The branch provider appends audit events for:

- `git.branch.create.requested`
- `policy.checked`
- `action.allowed` or `action.denied`
- `git.branch.create.started`
- `git.branch.create.completed` or `git.branch.create.failed`
- `evidence.created`
- `run.completed`

Invalid or protected branch names are rejected and recorded as denied audit events before execution.

## Known limits

This is a local branch creation slice only. It requires a local `git` executable and a Git working tree. If the branch already exists or Git refuses the branch, the provider records the nonzero exit and evidence.

At the Step 18b checkpoint, commit creation was deferred until Step 18c. Remote and PR behavior remains deferred until Step 19 or later.
