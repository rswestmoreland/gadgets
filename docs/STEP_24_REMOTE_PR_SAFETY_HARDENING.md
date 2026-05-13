# Step 24 - Remote PR Safety Hardening

Date: 2026-05-13

## Goal

Tighten the guarded remote pull request path before broader use, without adding broader Git remote behavior.

Step 24 keeps remote PR creation narrow:

- GitHub only
- disabled by default
- dry-run by default
- verified approval required
- local PR body evidence required
- configured base branch allowlist required
- configured head branch prefix allowlist required
- duplicate-open-PR handling required
- evidence and audit required

## Config shape

Generated `.gadgets/config.yaml` now includes:

```yaml
git:
  remote_pr:
    enabled: false
    dry_run: true
    provider: github
    owner: ""
    repo: ""
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
```

## Dry-run behavior

When `dry_run: true`, `gadgets git pr create` performs local validation and writes evidence, but it does not read the token environment variable and does not call the GitHub PR creation endpoint.

Dry-run mode still requires:

- `remote_pr.enabled: true`
- valid owner/repo config
- verified approval request
- unexpired approval record
- completed local PR body evidence
- base branch allowed by config
- head branch prefix allowed by config
- deterministic policy approval

## Execute behavior

When `dry_run: false`, the provider:

1. reads the configured token environment variable
2. checks for an existing open PR for the configured owner/head/base
3. applies `duplicate_strategy`
4. creates one GitHub pull request only when duplicate policy allows it
5. writes evidence and audit events

## Duplicate behavior

`duplicate_strategy` currently supports:

- `fail`: existing open PR is recorded as a failed run and no new PR is created
- `reuse`: existing open PR is recorded as the resulting PR and no new PR is created

## Preserved non-goals

Step 24 does not add:

- Git push
- Git fetch
- Git pull
- Git merge
- Git rebase
- Git checkout or switch
- remote branch creation
- GitLab support
- arbitrary shell
- provider-side tool execution
- Linux admin behavior
- database/cloud/deployment behavior

## Evidence

Remote PR evidence now records:

- dry-run state
- branch allowlist settings
- duplicate strategy
- duplicate-found status
- HTTP status when available
- PR number and URL when available
- redacted remote provider response

Token values are not written to evidence.
