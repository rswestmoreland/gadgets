# Capability Model

Capabilities are runtime-enforced permissions.

A Gadget can only request actions that match all of the following:

1. The Gadget has the named capability.
2. The action is inside the Gadget's zone and resource boundary.
3. The action is allowed by policy.
4. Required approval exists, if applicable.
5. The requested provider/tool is allowed.
6. Evidence requirements can be satisfied.

## Permission levels

- observe
- plan
- change
- release

## Naming convention

Use dotted names:

```text
domain.resource.action
```

Examples:

```text
repo.read
file.read
file.write
test.run
git.status
git.branch.create
git.commit.create
git.pr.create
linux.firewall.read
linux.firewall.apply
database.schema.read
database.migration.apply
```

Unknown capabilities are denied.

## Current enforcement baseline

The policy engine enforces declared capabilities, estimated permission level, tool allowlists, target zones, Safe Mode release blocking, filesystem path boundaries, denied paths, and approval-required decisions for mutating actions.

## `test.run` boundary

`test.run` is a change-level capability because executing tests launches a local process and may run project code.

Step 17 allows `test.run` only through the allowlisted Test Runner path:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The command string must come from `.gadgets/config.yaml` and must not be supplied by a model response or by free-form user text.

The Test Runner rejects unknown command names, empty command names, unsafe working directories, and parent traversal. Runtime policy checks `test.run` before execution.

`test.run` does not imply permission to apply patches, stage files, commit changes, create pull requests, run Linux admin actions, or execute arbitrary shell.

## `git.status` boundary

`git.status` is an observe-level capability for reading local working tree status. Step 18a allows it only through:

```bash
gadgets git status [--project <path>]
```

The runtime selects the fixed command `git status --short --branch --untracked-files=normal`. The model cannot supply Git arguments. The provider does not create branches, stage files, commit, push, pull, fetch, merge, rebase, create PRs, apply patches, run tests, or execute arbitrary shell.

## `git.branch.create` boundary

`git.branch.create` is a change-level capability because it mutates local Git refs.

Step 18b allows it only through:

```bash
gadgets git branch create [--project <path>] <branch-name>
```

The runtime selects the fixed command `git branch <validated-branch-name>`. The branch name is supplied as one CLI argument, validated by the runtime, checked against `.gadgets/config.yaml` `git.protected_branches`, and passed directly to `std::process::Command` without `sh -c`.

`git.branch.create` does not imply permission to checkout or switch branches, stage files, commit changes, push, pull, fetch, merge, rebase, create pull requests, apply patches, run tests, execute arbitrary shell, or perform Linux admin, database, cloud, or deployment actions.


## `git.commit.create` boundary

`git.commit.create` is a change-level capability because it mutates the local Git repository.

Step 18c allows it only through:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

The runtime verifies the approval request and approval record, verifies the exact `proposed.patch` hash and scope hash, extracts approved file paths from the patch artifact, rejects detached HEAD and protected current branches, rejects preexisting staged changes, stages only approved patch files, verifies the staged set, and creates one local commit.

`git.commit.create` does not imply permission to checkout or switch branches, push, pull, fetch, merge, rebase, create pull requests, apply patches, run tests, execute arbitrary shell, or perform Linux admin, database, cloud, or deployment actions.


## `git.pr.body.generate` capability

`git.pr.body.generate` is a local evidence-generation capability. It generates reviewable pull request title/body Markdown from a verified patch approval and optional evidence references.

Step 19 allows it only through:

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

The PR body provider must not create a remote PR, push, pull, fetch, merge, rebase, checkout, switch branches, stage files, create commits, apply patches, run tests, execute shell, or perform Linux admin/database/cloud/deployment behavior.

`git.pr.create` is implemented by the guarded remote PR provider. It is a change-level capability allowed only when remote PR creation is explicitly enabled in config, the approval is verified, the PR body evidence is present, and deterministic policy allows the action.
