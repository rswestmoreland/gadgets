# Roadmap

Date: 2026-05-12

## Progress summary

Use three scopes when describing progress:

| Scope | Current estimate | Notes |
|---|---:|---|
| Core safety spine through guarded remote PR creation | 100% | Core types, init, ledger, evidence, policy, manifest loading, providers, patch plan, approval records, approved patch apply, allowlisted test execution, local Git status, protected local branch creation, approved local commit scaffolding, local PR body generation, approval expiration enforcement, and guarded remote PR creation are implemented at checkpoint/code level. |
| Local Developer MVP | 96-97% | Remaining major items are external Rust validation, Documentation Gadget behavior, release packaging, and polish. |
| Full Gadgets Framework roadmap | 35-40% | Team workflows, Linux Server Admin packs, database/cloud/deployment packs, pack trust/signing, stronger secret handling, and UI/team integrations remain future work. |

## Phase 0 - Contract and skeleton

Status: mostly complete.

Completed:

- dual license selected: MIT OR Apache-2.0
- architecture baseline
- contract specs
- initial repository skeleton
- initial Rust workspace skeleton
- first vertical slice definition

Open:

- deeper crate split cleanup after contracts stabilize

## Phase 1 - Local observe runtime and provider-controlled handoffs

Status: complete for the current local baseline.

Completed:

- core contract types and manifest validation
- local `.gadgets/` init
- append-only audit ledger with hash-chain verification
- evidence bundle writer and verifier
- deterministic built-in policy checks
- observe-only Filesystem Read provider
- `gadgets ask <request>` local repo inspection flow
- mock provider trait integration in the ask flow
- richer Coordinator stub using structured handoff objects
- `.gadgets/config.yaml` loading
- configured provider profile selection
- runtime mode passed into policy evaluation
- pack/Gadget manifest loading from installed `.gadgets/` state and built-in manifests
- `gadgets pack list`
- `gadgets pack show`
- `gadgets pack validate`
- OpenAI provider adapter
- Anthropic provider adapter

## Phase 2 - Local developer change workflow

Status: active.

Completed:

- Patch Writer plan-only mode
- approval request/record scaffolding
- approved local patch apply
- allowlisted Test Runner
- local Git status
- protected local branch creation
- approved local commit scaffolding
- local PR body generation
- approval expiration enforcement

Next:

- Documentation Gadget executable behavior
- Dependency Gadget plan-only behavior
- onboarding and error-message polish

## Step 17 - Allowlisted Test Runner

Status: implemented at checkpoint/code level.

Goal:

Runs only named test commands configured in `.gadgets/config.yaml` through an explicit CLI path:

```bash
gadgets test run [--project <path>] <test-command-name>
```

Implemented boundary:

- no arbitrary shell
- no provider-side tool execution
- no model-supplied command string
- no user-supplied raw command string
- commands loaded only from `.gadgets/config.yaml`
- unknown command names rejected
- unsafe working directories rejected
- policy checks `test.run` before execution
- stdout, stderr, exit status, duration, and pass/fail captured as evidence
- audit events appended
- no patch application inside the Test Runner
- no Git/PR behavior in Step 17
- no Linux admin, database, cloud, or deployment behavior in Step 17

Recommended config shape:

```yaml
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
    timeout_seconds: 300
  - name: npm_test
    command: npm test
    working_dir: "."
    timeout_seconds: 300
```

## Phase 3 - Git and PR workflow

Status: active.

Completed:

- local observe-only Git status
- protected local branch creation
- approved local commit scaffolding

Planned:

- guarded remote PR creation behind explicit configuration
- Developer MVP release packaging and polish

## Step 18a - Local Git Status

Status: implemented at checkpoint/code level.

Implemented:

- `gadgets git status [--project <path>]`
- fixed local command: `git status --short --branch --untracked-files=normal`
- policy check for `git.status` before execution
- stdout/stderr/exit status/duration capture
- branch and changed-entry summary
- evidence bundle and audit events
- no branch, commit, remote, PR, shell, provider, patch, Linux admin, database, cloud, or deployment behavior

## Step 18b - Protected Local Branch Creation

Status: implemented at checkpoint/code level.

Implemented:

- `gadgets git branch create [--project <path>] <branch-name>`
- fixed local command: `git branch <validated-branch-name>`
- branch-name validation before execution
- protected branch config under `git.protected_branches`
- policy check for `git.branch.create` before execution
- stdout/stderr/exit status/duration capture
- evidence bundle and audit events
- no checkout, switch, stage, commit, remote, PR, shell, provider, patch, Linux admin, database, cloud, or deployment behavior

## Step 18c - Approved Local Commit Scaffolding

Status: implemented at checkpoint/code level.

Implemented:

- `gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]`
- verified approval request and approval record before staging or commit
- exact patch artifact hash and approval scope hash verification
- approved file extraction from `proposed.patch`
- protected current branch rejection through `git.protected_branches`
- detached HEAD rejection
- preexisting staged-change rejection
- staging only approved patch files
- staged-set verification before commit
- one fixed local `git commit` path through `std::process::Command` without `sh -c`
- evidence bundle and audit events
- no checkout, switch, push, pull, fetch, merge, rebase, PR, provider, patch apply, shell, tests, Linux admin, database, cloud, or deployment behavior

Next:

- external Rust validation and bounded fixes
- guarded remote PR creation behind explicit configuration

## Phase 4 - Linux Server Admin Observe Pack

Status: deferred.

Planned:

- host inventory
- process and port inspector
- service observe
- firewall planner
- cleanup planner
- package/patch planner
- backup/restore verifier

## Phase 5 - Team workflows

Status: deferred.

Planned:

- shared approvals
- ticketing
- notifications
- team policy profiles
- shared ledger export

## Phase 6 - Controlled production/change packs

Status: deferred.

Planned:

- database planner/executor
- cloud readonly/change planner
- deployment planner/executor
- Linux Server Admin Change Pack

## Current implemented local command surface

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
```

## Provider checkpoint

OpenAI and Anthropic provider adapters are implemented behind the provider trait. Mock remains default. Runtime-enforced policy/evidence/audit behavior is unchanged, and provider output remains untrusted structured Coordinator input.

## Step 16 checkpoint

Approved local patch application is implemented for exact approval-bound proposed patches. Patch application verifies the approval record, scope hash, patch SHA-256, supported unified diff format, and writable path policy before writing files. The allowlisted Test Runner, local Git status, protected local branch creation, approved local commit scaffolding, and local PR body generation are also implemented. The next recommended milestone is external Rust validation and bounded fixes.


## Step 19 completed checkpoint

Step 19 adds `gadgets git pr body`, a local-only PR body generator that verifies patch approval, summarizes patch scope, optionally references test and commit evidence, writes `pr_body.md` as evidence, and appends audit events. Step 21 adds guarded remote PR creation behind explicit config.


## Step 20 - Local Developer MVP Validation and Hardening

Status: implemented at checkpoint/code level.

Completed:

- approval expiration format locked to strict UTC RFC3339 without fractional seconds
- invalid expiration format rejected at approval request creation
- expired approval requests rejected before approval recording
- expired approvals rejected by approval verification
- CLI help updated for `--expires-at`
- local Developer MVP walkthrough added
- README, docs, specs, roadmap, and example config reconciled
- no remote behavior added

External validation remains required because Cargo is unavailable in this environment.


## Step 21 - Guarded remote PR creation

Status: implemented at checkpoint/code level.

Step 21 adds `gadgets git pr create`, guarded by explicit `git.remote_pr.enabled` config, verified patch approval, verified local PR body evidence, deterministic policy, and a token loaded from the configured environment variable. It creates one GitHub pull request through the GitHub API. It does not push branches, fetch, pull, merge, rebase, checkout, switch, run shell, call model-provider tools, apply patches, run tests, or perform Linux admin/database/cloud/deployment actions.

External Rust validation remains required.
