# Local Developer MVP Walkthrough

Date: 2026-05-13

This walkthrough describes the current local-only Developer Pack workflow.

For a broader alpha guide with configuration examples, troubleshooting, evidence/audit explanation, and known limitations, see `docs/DEVELOPER_MVP_ALPHA.md`.

It remains local-first by default. It does not push code, deploy software, administer Linux hosts, access databases, or call cloud APIs. Step 21 can create one GitHub pull request only when explicitly enabled in local config and tied to verified approval plus local PR-body evidence.

## Validated baseline

This walkthrough applies to the externally validated baseline:

```text
validated commit: c5fbd78
cargo fmt --check: PASS
cargo check: PASS
cargo test: PASS
cargo clippy --all-targets --all-features -- -D warnings: PASS
cargo build --release: PASS
```

## 1. Initialize local state

```bash
gadgets init
```

This creates `.gadgets/` with Safe Mode defaults, the Developer Pack selected, test commands disabled by default, and protected Git branches configured.

## 2. Inspect the repo

```bash
gadgets ask "Review this repo and explain how it is structured."
```

The runtime selects the configured provider, receives an untrusted Coordinator handoff, checks the Filesystem Read Gadget manifest and policy, reads allowed files, writes evidence, and appends audit events.

## 3. Generate a plan-only patch

```bash
gadgets ask "Propose a patch..."
```

Patch Writer creates a `proposed.patch` artifact as evidence only. No file is changed.

## 4. Request patch approval

```bash
gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z
```

The approval request binds to the exact `proposed.patch` SHA-256 and scope hash.

Expiration uses strict UTC RFC3339 without fractional seconds:

```text
YYYY-MM-DDTHH:MM:SSZ
```

## 5. Approve the request

```bash
gadgets approval approve <approval-request-id> <approver>
```

This records a scoped approval. It does not apply the patch.

## 6. Verify approval

```bash
gadgets approval verify <approval-request-id>
```

Verification checks request, approval record, patch hash, scope hash, status, and expiration.

## 7. Apply the approved patch

```bash
gadgets patch apply <approval-request-id>
```

Patch apply verifies approval and policy before any file write, prepares all target file changes before writing, writes evidence, and appends audit events.

It does not run tests, Git, PR, shell, provider tools, Linux admin, database, cloud, or deployment actions.

## 8. Run an allowlisted test command

First configure a named test command in `.gadgets/config.yaml`:

```yaml
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
    timeout_seconds: 300
```

Then run it by name:

```bash
gadgets test run cargo_test
```

The command string comes from config, not the prompt or model output.

## 9. Record local Git status

```bash
gadgets git status
```

This runs one fixed local status command and writes evidence.

## 10. Create a protected-safe local branch

```bash
gadgets git branch create work/example-change
```

The branch name is validated, protected branch patterns are rejected, and no checkout/switch occurs.

## 11. Commit the approved patch locally

```bash
gadgets git commit approved-patch <approval-request-id>
```

This verifies approval, rejects detached HEAD and protected current branches, rejects preexisting staged changes, stages only approved files, verifies the staged set, and creates one local commit.

It does not push, pull, fetch, merge, rebase, or create a PR.

## 12. Generate local PR body Markdown

```bash
gadgets git pr body <approval-request-id> --test-run <run-id> --commit-run <run-id>
```

This writes local Markdown evidence only. A separate Step 21 command can use that evidence to create one GitHub pull request if remote PR creation is explicitly enabled in config.

## 13. Verify evidence and ledger

```bash
gadgets evidence verify <run-id>
gadgets ledger verify
```

Use these after each meaningful run to confirm evidence metadata and audit hash chaining remain valid.


## Optional guarded remote PR creation

Remote PR creation is disabled by default. After generating PR body evidence, a configured project may run:

```bash
gadgets git pr create <approval-request-id> --body-run <pr-body-run-id> --head <branch> --base <branch>
```

This requires `git.remote_pr.enabled: true`, GitHub repository settings, allowed base/head branch config, duplicate-open-PR policy, and the configured token environment variable when `git.remote_pr.dry_run: false`. Generated config keeps `dry_run: true`; dry-run writes evidence without reading the token or making the GitHub mutation call. The head branch must already exist remotely because Gadgets does not push branches in this step.
