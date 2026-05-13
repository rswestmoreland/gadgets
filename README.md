# Gadgets Framework

Gadgets Framework is a safety-first, vendor-neutral framework for purpose-built AI workers called Gadgets.

A Gadget is a least-privilege AI worker that operates inside fixed capability zones and collaborates only through policy-enforced handoffs.

## Core rule

Models may reason, propose, summarize, and request actions.

Only the Gadgets runtime may authorize and execute actions.

Provider SDK behavior, prompts, model tool-calling, and agent handoff features are useful integration surfaces, but they are not the final security boundary.

## Initial product shape

The first implementation is a CLI-first local runtime focused on safe developer automation.

The current executable local workflow starts with observe-only repository inspection and supports plan-only patch proposals, scoped approvals with enforced expiration, approved local patch application, explicit allowlisted test runs, local Git status, protected local branch creation, approved local commits, and local PR body generation:

```bash
gadgets ask "Review this repo and explain how it is structured."
gadgets ask "Propose a patch..."
gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z
gadgets approval approve <approval-request-id> <approver>
gadgets patch apply <approval-request-id>
gadgets test run <test-command-name>
gadgets git status
gadgets git branch create <branch-name>
gadgets git commit approved-patch <approval-request-id>
gadgets git pr body <approval-request-id>
```

Expected observe behavior:

1. `gadgets ask` loads `.gadgets/config.yaml`.
2. The configured model provider profile is selected.
3. The provider returns a structured Coordinator handoff request.
4. The runtime validates the handoff against installed packs and loaded Gadget manifests.
5. The Filesystem Read Gadget inspects allowed repository paths.
6. Policy denies secret and protected paths.
7. Evidence bundle is produced.
8. Audit ledger records run, provider response, handoff, actions, denied accesses, and evidence.
9. No files are modified and no commands are executed.

Expected patch behavior:

1. Patch Writer creates a plan-only `proposed.patch` evidence artifact.
2. Approval records bind to the exact `proposed.patch` SHA-256, deterministic scope hash, and optional strict UTC expiration.
3. `gadgets patch apply` verifies the approval request, approval record, expiration, scope hash, patch hash, patch format, policy, writable paths, and denied paths.
4. Patch application prepares all target file changes before writing any file.
5. Patch application writes its own evidence bundle and audit events.
6. Patch application does not run tests, Git commands, PR behavior, shell commands, Linux admin actions, database actions, cloud actions, or deployment actions.

## Implemented provider profiles

The mock provider remains the default.

Optional live providers now available behind the same `ModelProvider` trait:

- `openai`
- `anthropic`

Provider output is always treated as an untrusted structured request. It cannot bypass Gadget manifests, pack validation, policy checks, evidence generation, or audit logging.

## Locked Phase 0 direction

- Rust core runtime.
- CLI-first MVP.
- Provider-neutral model adapters.
- Mock provider default.
- OpenAI provider opt-in.
- Anthropic provider opt-in.
- Runtime policy remains the authority after provider handoffs.
- YAML manifests/config.
- JSONL audit/event streams.
- Built-in deterministic policy checks first.
- Safe Mode default.
- Developer Pack first.
- Linux Server Admin Observe Pack before Change Pack.
- No generic root-shell Gadget.
- Approval required for file writes in v0.1; approval expiration is enforced when present; Step 18b permits only validated non-protected local branch ref creation, Step 18c permits approved local commits scoped to verified patch approvals, and Step 19 generates local PR Markdown evidence only.
- Evidence and audit required for meaningful work.
- No arbitrary shell in the MVP.
- Allowlisted test commands are implemented through `gadgets test run` using named configured commands only.
- Local Git status is implemented through `gadgets git status` using one fixed runtime-selected observe command.
- Protected local branch creation is implemented through `gadgets git branch create` using one fixed runtime-selected local branch command.
- Approved local commit creation is implemented through `gadgets git commit approved-patch` using verified patch approval scope and fixed Git commands.
- Local PR body generation is implemented through `gadgets git pr body` using verified approval and optional evidence references.
- Approval records can be created for plan-only patch artifacts.
- Approved local patch application is implemented through `gadgets patch apply`, with exact approval scope/hash verification before any file write.

## License and author

Gadgets Framework is dual-licensed under MIT OR Apache-2.0, at your option.

- MIT License: see `LICENSE-MIT`
- Apache License, Version 2.0: see `LICENSE-APACHE`
- Dual-license summary: see `LICENSE.md`

Author: Richard S. Westmoreland <dev@rswestmore.land>

Copyright 2026 Richard S. Westmoreland

## Repository layout

```text
docs/       Architecture, decisions, implementation plan, roadmap.
specs/      Contract specs for manifests, capabilities, zones, handoffs, evidence, audit, providers, and packs.
crates/     Rust workspace crates.
packs/      Built-in Gadget pack manifests and Gadget manifests.
examples/   Example projects and local `.gadgets/` configuration.
```

## Current implemented commands

```bash
gadgets init [path]
gadgets ask [--project <path>] <request>
gadgets ledger show [project-root-or-ledger-path]
gadgets ledger verify [project-root-or-ledger-path]
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
gadgets pack validate [--project <path>] [--strict] [pack]
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]
gadgets approval approve [--project <path>] <approval-request-id> <approver>
gadgets approval show [--project <path>] <approval-request-id>
gadgets approval verify [--project <path>] <approval-request-id>
gadgets approval id-for-run <run-id>
gadgets patch apply [--project <path>] <approval-request-id>
gadgets test run [--project <path>] <test-command-name>
gadgets git status [--project <path>]
gadgets git branch create [--project <path>] <branch-name>
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

## Allowlisted test command workflow

Step 17 implements an allowlisted Test Runner:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The Test Runner runs only named test commands configured in `.gadgets/config.yaml`. It does not accept arbitrary command strings from a model response or from free-form user input. It launches commands directly without `sh -c`, checks `test.run` through deterministic policy, captures stdout/stderr/exit status/duration, writes evidence, and appends audit events.

## Current status

Step 21 adds guarded remote PR creation after Step 20 local Developer MVP hardening. The current Developer Pack workflow can inspect a repo, propose a patch, create and approve a scoped patch approval record, apply the exact approved patch when policy and hash checks pass, run a named configured test command, record local Git status evidence, create a validated non-protected local branch, create one local commit from approved patch files on a non-protected branch, generate a reviewable local PR body Markdown artifact, and optionally create one GitHub pull request when remote PR creation is explicitly enabled in config.

Still not implemented:

- external Rust validation in this environment
- arbitrary shell execution
- Linux server administration actions
- database/cloud/deployment behavior
- rollback execution
- remote Git behavior such as push, pull, fetch, merge, or rebase


## Local Git workflow

Step 18a implements observe-only local Git status:

```bash
gadgets git status [--project <path>]
```

The Git status provider runs only one fixed command selected by the runtime: `git status --short --branch --untracked-files=normal`. It launches the command directly without `sh -c`, checks `git.status` through deterministic policy, captures stdout/stderr/exit status/duration, writes evidence, and appends audit events.

Step 18b implements protected local branch creation:

```bash
gadgets git branch create [--project <path>] <branch-name>
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

The branch provider runs only one fixed command selected by the runtime: `git branch <validated-branch-name>`. The branch name is supplied as a single CLI argument, validated by the runtime, checked against `git.protected_branches` in `.gadgets/config.yaml`, and passed directly to `std::process::Command` without `sh -c`.

This does not checkout or switch branches, stage files, commit, push, pull, fetch, merge, rebase, create PRs, call providers, apply patches, run tests, or perform admin actions. Secret-like Git output lines are redacted before evidence write.

Step 18c implements approved local commit creation:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

The commit provider verifies the approval request and approval record, verifies the exact patch artifact hash and scope hash, extracts the approved file list from `proposed.patch`, rejects detached HEAD and protected current branches, rejects preexisting staged changes, stages only files named by the approved patch, verifies the staged set, and creates one local commit. It uses `std::process::Command` without `sh -c`. If the commit fails after staging, it attempts a fixed best-effort `git reset -- <approved-files>` cleanup.

This does not checkout or switch branches, push, pull, fetch, merge, rebase, create PRs, call providers, apply patches, run tests, or perform admin actions.

Step 19 implements local PR body generation:

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

The PR body generator verifies the approval request and approval record, summarizes the approved patch, optionally references prior test and commit evidence bundles, and writes `pr_body.md` as evidence. It does not create a remote PR, push, pull, fetch, merge, rebase, call providers, apply patches, run tests, execute shell, or perform admin actions. Step 21 adds the separate `gadgets git pr create` path for explicitly configured remote PR creation.


Step 21 implements guarded remote PR creation:

```bash
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

Remote PR creation is disabled by default. To enable it, `.gadgets/config.yaml` must set `git.remote_pr.enabled: true` and configure `provider: github`, `owner`, `repo`, `api_base`, `token_env`, and `default_base_branch`. The provider verifies the approval request, approval record, approval expiration, local PR body evidence run, deterministic policy, and configured remote PR settings before making one GitHub API call to create a pull request. It does not push branches; the head branch must already exist remotely. It does not run `git push`, `git fetch`, `git pull`, merge, rebase, checkout, switch, shell, provider tools, patch apply, tests, Linux admin, database, cloud, or deployment actions. The token value is loaded from the configured environment variable and is not written to evidence.

## Current local patch workflow

The current Developer Pack workflow can inspect a repo, produce a plan-only patch, create and approve a patch approval record with optional strict UTC expiration, and apply the exact approved patch through `gadgets patch apply`. Patch application verifies the approval record, expiration, scope hash, patch SHA-256, and writable path policy before writing files. It does not run shell commands, tests, Git, PR, Linux admin, database, cloud, or deployment actions.

## Step 17 Test Runner boundary

The Test Runner implementation is explicit and narrow:

- `gadgets test run [--project <path>] <test-command-name>`
- command string loaded only from `.gadgets/config.yaml`
- unknown command names rejected
- empty command names rejected
- unsafe working directories rejected
- parent traversal rejected
- `test.run` policy checked before execution
- command execution launched directly without `sh -c`
- stdout/stderr/exit status/duration captured as evidence
- audit events appended
- no patch application inside the Test Runner
- stdout/stderr are capped and secret-like output lines are redacted before evidence write
- remote PR creation is disabled by default and requires explicit `git.remote_pr.enabled` config
- no Linux admin, database, cloud, deployment, or production behavior

## Approval expiration

Patch approval requests may include an expiration:

```bash
gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z
```

The timestamp must use strict UTC RFC3339 without fractional seconds: `YYYY-MM-DDTHH:MM:SSZ`. Expiration is validated when the request is created, rejected if already expired when approval is recorded, and checked again by approval verification. Patch apply, approved local commit, and local PR body generation all rely on approval verification.

See `docs/LOCAL_DEVELOPER_MVP_WALKTHROUGH.md` for the current local workflow.

## External validation

If the Rust toolchain is available, validate with:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
