# Developer MVP Alpha Guide

Date: 2026-05-13

## Purpose

This guide describes the current Developer MVP alpha for Gadgets Framework.

The alpha is a local-first developer workflow with optional guarded GitHub pull request creation. It is intended to demonstrate the core safety model: models can propose and summarize, but the runtime authorizes and executes actions through policy, evidence, and audit controls.

## Validated baseline

This guide applies to the externally validated baseline:

```text
gadgets-main.zip
validated commit: c5fbd78
validation status: passed end-to-end
```

Validation passed:

```text
cargo fmt --check                                      PASS
cargo check                                           PASS
cargo test                                            PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo build --release                                 PASS
```

Rust/Cargo versions reported for the validation run:

```text
rustc 1.89.0 (29483883e 2025-08-04)
cargo 1.89.0 (c24e10642 2025-06-23)
```

## What this alpha can do today

The current Developer MVP can:

- initialize local `.gadgets/` project state with Safe Mode defaults
- inspect allowed repository files through the Filesystem Read Gadget
- route provider output through untrusted Coordinator handoffs
- generate plan-only patch evidence
- create scoped patch approval requests
- record human approval records
- verify approvals, patch hashes, scope hashes, and expiration
- apply the exact approved local patch
- run named allowlisted test commands from `.gadgets/config.yaml`
- record fixed local Git status evidence
- create a validated non-protected local branch
- create one local commit from approved patch files
- generate local PR body Markdown evidence
- optionally create one GitHub pull request when remote PR creation is explicitly enabled
- write evidence bundles for meaningful work
- append audit ledger events and verify the audit hash chain

## What this alpha intentionally cannot do

The current Developer MVP does not implement:

- arbitrary shell execution
- model-selected raw commands
- generic root-shell behavior
- provider-side tool execution bypass
- Git push
- Git fetch, pull, merge, or rebase
- Git checkout or switch
- remote branch creation
- GitLab or Bitbucket PR/MR creation
- Linux server administration behavior
- database behavior
- cloud behavior
- deployment behavior
- full secret scanning or DLP
- pack signing or trust roots
- Team Mode approval workflows

## Core safety model

The alpha keeps broad reasoning separate from narrow authority:

1. A model or mock provider can propose a structured handoff.
2. The runtime loads the target Gadget manifest.
3. The policy engine checks capability, tool, zone, path, runtime mode, and approval context.
4. The tool provider executes only its fixed, narrow action.
5. Evidence is written for meaningful work.
6. Audit events are appended.

Provider SDK behavior, model tool-calling, prompt instructions, and handoff features are integration surfaces. They are not the final security boundary.

## Minimal setup

Initialize a project:

```bash
gadgets init
```

This creates local `.gadgets/` state, including:

```text
.gadgets/config.yaml
.gadgets/ledger/events.jsonl
.gadgets/runs/
.gadgets/approvals/
```

The default config uses Safe Mode, the deterministic mock provider, the Developer Pack, denied secret/protected paths, empty test commands, protected Git branches, and remote PR creation disabled.

## Sample local config

A minimal local config shape:

```yaml
schema_version: gadgets.framework/config/v0.1

mode: safe
default_model_profile: mock_default

model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock

installed_packs:
  - developer

zones:
  local_repo:
    type: local_repo
    root: "."
    readable_paths:
      - "."
    writable_paths: []
    denied_paths:
      - ".git/"
      - ".gadgets/"
      - ".env"
      - "secrets/"
      - "**/*.pem"
      - "**/*.key"
      - "**/*secret*"
      - "**/*credential*"

audit:
  ledger_path: ".gadgets/ledger/events.jsonl"

evidence:
  root: ".gadgets/runs"

approval:
  require_for_all_writes: true

test_commands: []

git:
  protected_branches:
    - main
    - master
    - trunk
    - production
    - prod
    - release/
  remote_pr:
    enabled: false
    provider: github
    owner: ""
    repo: ""
    api_base: https://api.github.com
    token_env: GITHUB_TOKEN
    default_base_branch: main
```

## Test command example

The Test Runner runs only named commands configured in `.gadgets/config.yaml`.

Example:

```yaml
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
    timeout_seconds: 300
```

Run by name:

```bash
gadgets test run cargo_test
```

The runtime rejects unknown command names, parent traversal in `working_dir`, absolute working directories, shell metacharacters, unsupported quoting, and raw command strings supplied by prompts or model output.

## Remote PR config example

Remote PR creation is disabled by default. Dry-run mode is also enabled by default. Enable it only in a local project where GitHub PR creation is intended, and keep dry-run on until the evidence output is reviewed:

```yaml
git:
  remote_pr:
    enabled: true
    dry_run: true
    provider: github
    owner: example-owner
    repo: example-repo
    api_base: https://api.github.com
    token_env: GITHUB_TOKEN
    default_base_branch: main
    allowed_base_branches:
      - main
    allowed_head_prefixes:
      - feature/
      - fix/
      - docs/
    duplicate_strategy: fail
  protected_branches:
    - main
    - master
    - trunk
    - production
    - prod
    - release/
```

Do not put token values in the config file. Set the environment variable named by `token_env` outside the repository. Dry-run mode does not read the token or make the GitHub mutation call. To create a PR, set `dry_run: false` after reviewing the dry-run evidence.

Remote PR creation does not push branches. The head branch must already exist remotely and must match one of `allowed_head_prefixes`. The base branch must be listed in `allowed_base_branches`. Duplicate open PR behavior is controlled by `duplicate_strategy`, currently `fail` or `reuse`.

## Complete alpha workflow

Inspect the repository:

```bash
gadgets ask "Review this repo and explain how it is structured."
```

Generate a plan-only patch:

```bash
gadgets ask "Propose a patch that updates docs/CHANGE_TARGET.md"
```

Request approval for the patch plan run:

```bash
gadgets approval request-patch <patch-plan-run-id> --expires-at 2999-01-01T00:00:00Z
```

Approve the request:

```bash
gadgets approval approve <approval-request-id> "Richard S. Westmoreland"
```

Verify the approval:

```bash
gadgets approval verify <approval-request-id>
```

Apply the exact approved patch:

```bash
gadgets patch apply <approval-request-id>
```

Run a configured test command:

```bash
gadgets test run cargo_test
```

Record Git status evidence:

```bash
gadgets git status
```

Create a local non-protected branch:

```bash
gadgets git branch create work/example-change
```

Commit only approved patch files:

```bash
gadgets git commit approved-patch <approval-request-id> --message "Update change target"
```

Generate local PR body evidence:

```bash
gadgets git pr body <approval-request-id> --test-run <test-run-id> --commit-run <commit-run-id> --title "Update change target"
```

Optionally create a GitHub PR when explicitly enabled:

```bash
gadgets git pr create <approval-request-id> --body-run <pr-body-run-id> --head work/example-change --base main --title "Update change target"
```

Verify evidence and ledger:

```bash
gadgets evidence verify <run-id>
gadgets ledger verify
```

## Evidence and audit

Meaningful work writes evidence under `.gadgets/runs/` and appends audit events under `.gadgets/ledger/events.jsonl`.

Evidence bundles normally include:

- `bundle.yaml`
- `summary.md`
- action-specific artifacts
- artifact SHA-256 hashes
- bundle metadata hash

The audit ledger is append-only JSONL with event hashes and previous-event hash chaining. Use `gadgets ledger verify` to detect ledger tampering.

## Troubleshooting

### Missing `.gadgets/config.yaml`

Run:

```bash
gadgets init
```

### Unknown test command

Add a named command under `test_commands` in `.gadgets/config.yaml`, then run it by name.

### Test command rejected for shell metacharacters

The Test Runner is intentionally narrow. Use a simple command shape such as `cargo test` or `npm test`. Do not use pipes, redirects, chained commands, command substitution, or shell quoting.

### Patch approval expired

Create a new approval request with a future expiration timestamp:

```text
YYYY-MM-DDTHH:MM:SSZ
```

### Commit rejected because the current branch is protected

Create a non-protected branch first, then switch to it manually outside Gadgets. Gadgets can create a branch, but it does not checkout or switch branches.

### Commit rejected because files are already staged

Clear or commit the existing staged changes outside Gadgets, then retry. Gadgets stages only approved patch files and rejects preexisting staged changes.

### Remote PR creation failed because the head branch is missing

Push the branch outside Gadgets or use your normal Git workflow. Gadgets does not push branches in this alpha.

### GitHub token missing

Set the environment variable named by `git.remote_pr.token_env`. Do not put token values in config, docs, evidence, or prompts.

## Known limitations

- Test processes are not OS-sandboxed.
- Output redaction is centralized and best-effort, but not full DLP or complete secret scanning.
- Remote PR creation is GitHub-only.
- Pack trust/signing is not implemented.
- Team approval workflows are not implemented.

## Recommended next step

Proceed with pack trust/signing design before broader third-party pack or Team workflows.
