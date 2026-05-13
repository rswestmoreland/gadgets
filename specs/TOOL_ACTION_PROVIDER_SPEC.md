# Tool and Action Provider Specification

Tool/action providers are the non-model execution layer used by Gadgets.

No tool executes just because a model requested it.

Every meaningful tool call must pass runtime checks and produce audit/evidence where applicable.

## Authority boundary

Provider SDKs and model tool-calling APIs are not security boundaries. They may be used as integration surfaces, but runtime-enforced Gadget manifests, capabilities, policy, evidence, and audit remain authoritative.

## Implemented providers and runtime slices

- observe-only Filesystem Read provider
- Patch Writer plan-only provider
- approved local patch apply provider
- allowlisted Test Runner provider
- local Git status provider
- protected local Git branch provider
- approved local Git commit provider
- local PR body generator
- audit ledger writer
- evidence bundle writer
- mock model provider
- OpenAI model provider adapter
- Anthropic model provider adapter

## Allowlisted Test Runner provider

Status: implemented at checkpoint/code level.

The allowlisted Test Runner runs only configured test commands from `.gadgets/config.yaml`.

CLI entrypoint:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The Test Runner must reject:

- unknown command names
- empty command names
- command strings supplied by the model
- command strings supplied directly in a free-form user request
- working directories outside the project boundary
- parent traversal in `working_dir`
- shell-style command composition that would turn the feature into arbitrary shell

The Test Runner captures:

- configured command name
- resolved working directory
- stdout
- stderr
- exit code
- duration if practical
- pass/fail result
- policy decision

The Test Runner writes evidence and appends audit events. It launches commands directly with `std::process::Command` and does not use `sh -c`.

It must not:

- apply patches
- stage files
- commit files
- create branches
- open pull requests
- run Linux admin actions
- run database/cloud/deployment actions
- call provider-side tools

## Deferred providers

- process inspector
- service manager
- firewall
- package manager
- container runtime
- database
- cloud

## Step 6 implementation note

The policy engine can deny requests for tools not allowlisted by the Gadget manifest before a provider is invoked.

## Filesystem Read provider

The observe-only Filesystem Read provider evaluates directory traversal with `file.search` and file reads with `file.read` through policy before touching the target path.

It must not modify files or execute commands.

## Patch plan provider

The `patch.plan` tool may create a proposed patch artifact as evidence.

It must not modify files, apply patches, stage changes, commit, run tests, or open pull requests.

## Step 16 patch apply provider

The Patch Writer apply provider may apply a local text patch only after a verified approval record and exact patch hash binding. Each target path must pass deterministic policy with approval present.

The provider does not run shell commands, tests, Git commands, provider tools, PR actions, Linux admin actions, database actions, cloud actions, or deployment actions.

## Local Git status provider

Step 18a adds a fixed local observe provider for `git.status`. It runs `git status --short --branch --untracked-files=normal` through `std::process::Command` without `sh -c`. It does not accept arbitrary Git arguments and does not perform branch, commit, staging, push, pull, fetch, merge, rebase, PR, provider, patch, shell, Linux admin, database, cloud, or deployment behavior.

The provider writes evidence and audit events for every status run that reaches execution.

## Protected local Git branch provider

Step 18b adds a fixed local provider for `git.branch.create`. It runs `git branch <validated-branch-name>` through `std::process::Command` without `sh -c`. It does not accept arbitrary Git arguments and does not perform checkout, switch, staging, commit, push, pull, fetch, merge, rebase, PR, provider, patch, shell, Linux admin, database, cloud, or deployment behavior.

The requested branch name must pass runtime validation and must not match `.gadgets/config.yaml` `git.protected_branches`. Protected branch entries can be exact names such as `main` or prefix entries ending in `/`, such as `release/`.

The provider writes evidence and audit events for every branch create run that reaches execution.


## Approved local Git commit provider

Step 18c adds a fixed local provider for `git.commit.create`. It runs only approval-backed local commit scaffolding through `std::process::Command` without `sh -c`.

The provider must:

- verify the approval request and approval record
- verify the exact patch artifact hash and scope hash
- extract approved file paths from `proposed.patch`
- reject detached HEAD
- reject protected current branches from `.gadgets/config.yaml` `git.protected_branches`
- reject preexisting staged changes
- stage only approved patch files
- verify the staged set before commit
- create at most one local commit
- write evidence and audit events

The provider must not accept arbitrary Git arguments, checkout or switch branches, push, pull, fetch, merge, rebase, create PRs, call provider-side tools, apply patches, run tests, execute shell, or perform Linux admin, database, cloud, or deployment behavior.


## Local PR body generator

Step 19 adds a local provider for `git.pr.body.generate`. It verifies a patch approval, summarizes the approved patch, optionally references test and commit evidence bundles, and writes reviewable PR body Markdown as evidence.

The PR body provider must not create remote PRs, push, pull, fetch, merge, rebase, checkout, switch branches, stage files, create commits, apply patches, run tests, execute shell, call model providers, or perform Linux admin, database, cloud, or deployment behavior. The separate remote PR provider may create one GitHub pull request only when explicitly enabled in config and tied to verified approval plus local PR-body evidence.


### Guarded remote PR provider

`git.pr.create` is implemented as a fixed GitHub API path, not as arbitrary shell or model tool execution. It requires `git.remote_pr.enabled: true`, configured repository owner/name, a verified approval request, unexpired approval, a completed PR-body evidence bundle, configured base/head branch constraints, duplicate-open-PR handling, and deterministic policy approval. Generated config keeps `git.remote_pr.dry_run: true`; in dry-run mode the provider writes evidence without making the GitHub mutation call or reading the token. When `dry_run: false`, it reads the configured token environment variable, checks for an existing open PR, and creates one GitHub PR only if duplicate policy allows it. It does not push branches; the head branch must already exist remotely. It does not run Git push, pull, fetch, merge, rebase, checkout, switch, patch apply, tests, Linux admin, database, cloud, or deployment actions.
