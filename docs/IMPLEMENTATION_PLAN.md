# Implementation Plan

Date: 2026-05-12

## Build principle

Build the runtime safety skeleton before adding powerful Gadgets.

The first goal is not autonomy. The first goal is a trustworthy, auditable authority boundary where provider output remains untrusted and the Gadgets runtime authorizes every action.

## Completed steps

### Step 1 - Skeleton and docs

Status: complete.

- Root README.
- Architecture docs.
- Phase 0 baseline.
- Decision record.
- Roadmap.
- Contract specs.
- Built-in pack placeholders.
- Minimal Rust workspace skeleton.

### Step 2 - Core data types and manifest parsing

Status: complete.

- Gadget and Pack manifest structs.
- Capability names.
- Permission levels.
- Zones and boundaries.
- Handoffs.
- Action requests.
- Policy decisions.
- Evidence metadata.
- Audit metadata.
- Validation reports and errors.

### Step 3 - Local project init

Status: complete.

Implemented:

```bash
gadgets init [path]
```

Creates local `.gadgets/` project state with Safe Mode, mock provider defaults, Developer Pack selection, denied secret/protected paths, and approval-required write posture.

### Step 4 - Append-only audit ledger

Status: complete.

Implemented:

```bash
gadgets ledger show [project-root-or-ledger-path]
gadgets ledger verify [project-root-or-ledger-path]
```

Includes JSONL events, SHA-256 event hashes, previous-event hash chaining, verification, and append refusal when the existing ledger is invalid.

### Step 5 - Evidence bundles

Status: complete.

Implemented:

```bash
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
```

Includes per-run evidence directories, `bundle.yaml`, `summary.md`, optional artifacts, artifact hashes, and bundle metadata hashes.

### Step 6 - Deterministic policy engine v0.1

Status: complete.

Implemented checks for declared capabilities, permission levels, tool allowlists, zones, filesystem paths, denied paths, readable/writable boundaries, Safe Mode/Team Mode restrictions, and approval-required mutating actions.

### Step 7 - Observe-only Filesystem Read Gadget

Status: complete.

Implemented observe-only repo inspection through:

```bash
gadgets ask <request>
```

The Filesystem Read slice routes file search/read actions through policy, writes evidence, and appends audit events. It does not modify files or execute commands.

### Step 8 - Mock provider and Coordinator stub

Status: complete.

Added deterministic `MockProvider` and a Coordinator stub that emits structured handoffs. Provider output remains untrusted and must pass runtime validation.

### Step 9 - Config loading and provider profile selection

Status: complete.

`gadgets ask` now loads `.gadgets/config.yaml`, selects `default_model_profile`, validates provider support, and passes configured runtime mode into policy evaluation.

### Step 10 - Pack and Gadget manifest loading

Status: complete.

Implemented project-local and built-in pack/Gadget manifest loading, plus:

```bash
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
```

### Step 11 - Pack validation

Status: complete.

Implemented:

```bash
gadgets pack validate [--project <path>] [--strict] [pack]
```

Validates pack manifests, declared Gadget manifests, missing/invalid Gadgets, manifest name mismatches, and pack highest-permission constraints.

### Step 12 - OpenAI provider adapter

Status: complete.

Added `OpenAiProvider` behind the provider trait. OpenAI can propose structured Coordinator handoffs but cannot execute tools, read files directly, approve actions, or mutate state.

### Step 13 - Anthropic provider adapter

Status: complete.

Added `AnthropicProvider` behind the provider trait. Anthropic can propose structured Coordinator handoffs but cannot execute tools, read files directly, approve actions, or mutate state.

### Step 14 - Patch Writer plan-only mode

Status: complete.

Patch Writer can produce a `proposed.patch` evidence artifact through a policy-checked plan action. It does not apply patches, write files, run commands, stage files, commit, or open PRs.

### Step 15 - Approval record scaffolding

Status: complete.

Implemented:

```bash
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]
gadgets approval approve [--project <path>] <approval-request-id> <approver>
gadgets approval show [--project <path>] <approval-request-id>
gadgets approval verify [--project <path>] <approval-request-id>
gadgets approval id-for-run <run-id>
```

Approvals bind to the exact proposed patch hash and deterministic scope hash.

### Step 16 - Approved local patch application

Status: complete.

Implemented:

```bash
gadgets patch apply [--project <path>] <approval-request-id>
```

Patch application requires:

- valid approval request
- valid approval record
- matching scope hash
- matching proposed patch SHA-256
- parseable supported unified diff
- each target path allowed by deterministic policy with approval present
- all target file changes prepared before any file write

Evidence includes `applied.patch`, `files_changed.txt`, `before_after_hashes.txt`, `approval_verification.txt`, `policy_decisions.txt`, assumptions, summary, and bundle metadata.

Boundary:

- No shell commands.
- No tests.
- No Git commands.
- No PR creation.
- No provider-side tool execution.
- No Linux admin, database, cloud, or deployment actions.

## Next step

### Step 17 - Allowlisted Test Runner

Status: implemented at checkpoint/code level.

Goal: run only named test commands configured in `.gadgets/config.yaml` through an explicit CLI command:

```bash
gadgets test run [--project <path>] <test-command-name>
```

Implementation requirements:

- Runtime config support for `test_commands` added.
- `test.run` capability and Test Runner manifest path verified.
- Narrow Test Runner provider added.
- `gadgets test run [--project <path>] <test-command-name>` added.
- Unknown command names rejected.
- Empty command names rejected.
- Model-supplied command strings rejected by design; only config names are accepted.
- Free-form user command strings rejected by CLI shape.
- Unsafe working directories and parent traversal rejected.
- Commands execute without `sh -c`.
- Deterministic policy evaluation for `test.run` runs before execution.
- stdout/stderr/exit status captured.
- Duration captured.
- Pass/fail recorded.
- Test evidence written separately from patch apply evidence.
- Audit events appended.
- Test running remains separate from patch application.
- No Git/PR, Linux admin, database, cloud, deployment, or production behavior added.

Acceptance criteria:

- [x] A named allowlisted test command can run.
- [x] Unknown command names are rejected.
- [x] Commands are loaded from `.gadgets/config.yaml`.
- [x] Arbitrary command strings are not accepted.
- [x] `working_dir` is constrained to the project boundary.
- [x] Parent traversal is rejected.
- [x] Runtime policy checks `test.run` before execution.
- [x] stdout/stderr/exit status are captured.
- [x] Evidence bundle is written.
- [x] Audit ledger events are appended.
- [x] Test failure is recorded without hiding output.
- [x] No patch application happens inside the Test Runner.

## Deferred

- Remote PR creation and remote Git behavior.
- Documentation Gadget executable behavior.
- Dependency Gadget.
- Linux Server Admin Observe Pack implementation.
- Linux Server Admin Change Pack implementation.
- Database, cloud, and deployment packs.
- Arbitrary shell.
- Public pack registry.
- Pack signing/trust model.

## Step 18a - Local Git status

Status: implemented at checkpoint/code level.

Implemented command:

```bash
gadgets git status [--project <path>]
```

Scope: fixed observe-only local Git status command with policy, evidence, and audit. No arbitrary Git arguments, shell, branch creation, commits, staging, remote operations, PR creation, provider tools, patch application, tests, Linux admin, database, cloud, or deployment behavior.

Step 18b later added protected local branch creation, Step 18c added approved local commit scaffolding, Step 19 added local PR body generation, and Step 21 added guarded remote PR creation behind explicit configuration.


## Step 18c - Approved local commit scaffolding

Status: implemented at checkpoint/code level.

Implemented command:

```bash
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

Scope: verify a scoped patch approval, reject protected current branches and detached HEAD, reject preexisting staged changes, stage only approved patch files, verify the staged set, and create one local commit. No checkout, switch, push, pull, fetch, merge, rebase, PR, provider, patch apply, shell, test, Linux admin, database, cloud, or deployment behavior.

Step 19 added PR body generation only. Step 21 later added guarded remote PR creation behind explicit configuration.


## Step 19 - Local PR body generation

Status: complete at checkpoint/code level.

Scope: generate reviewable local pull request Markdown from a verified patch approval plus optional test and commit evidence references. The command is:

```bash
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

The provider verifies the approval request and approval record, summarizes approved patch files, writes `pr_body.md` as evidence, and appends audit events. It does not create remote PRs, push, pull, fetch, merge, rebase, checkout, switch, stage, commit, apply patches, run tests, execute shell, call providers, or perform Linux admin/database/cloud/deployment behavior. Step 21 adds the separate guarded remote PR creation provider.

Step 21 later added guarded remote PR creation behind explicit config; external Rust validation remains required afterward.


## Step 20 - Local Developer MVP hardening

Status: implemented at checkpoint/code level.

Scope:

- Locked approval expiration format to strict UTC RFC3339 without fractional seconds.
- Validated expiration format when creating patch approval requests.
- Rejected expired approval requests before approval recording.
- Rejected expired approvals during approval verification.
- Preserved existing approval-backed enforcement in patch apply, approved local commit, and local PR body generation.
- Added a Local Developer MVP walkthrough.
- Reconciled README, roadmap, specs, example config, and file manifest.

No Git push/pull/fetch/merge/rebase, arbitrary shell, Linux admin, database, cloud, or deployment behavior was added.

External Rust validation is still required.


## Step 21 - Guarded remote PR creation

Status: implemented at checkpoint/code level.

Implemented command:

```bash
gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]
```

Scope: create one GitHub pull request only when remote PR creation is explicitly enabled in `.gadgets/config.yaml`, a patch approval is verified, approval expiration has not passed, a completed PR body evidence run is supplied, deterministic policy allows `git.pr.create`, and a token is available from the configured environment variable. The provider does not push branches; the head branch must already exist remotely. No Git push, pull, fetch, merge, rebase, checkout, switch, arbitrary shell, provider-side tool execution, patch apply, tests, Linux admin, database, cloud, or deployment behavior is added.

External Rust validation is still required.
