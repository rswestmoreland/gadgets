# Gadgets Framework - Step 17 Kickoff Prompt

You are continuing work on the Gadgets Framework project.

## Project summary

Gadgets Framework is a safety-first, vendor-neutral framework for purpose-built AI workers called Gadgets.

A Gadget is a least-privilege AI worker that operates inside fixed capability zones and collaborates only through policy-enforced handoffs.

The core rule is:

> Models may reason, propose, summarize, and request actions. Only the Gadgets runtime may authorize and execute actions.

Provider SDK behavior, prompts, model tool calling, and agent handoffs are integration surfaces, not final security boundaries.

## Current authoritative checkpoint

Use the reviewed Step 16 wrap-up bundle as the current baseline:

```text
gadgets-framework-step16-reviewed-wrapup-v0_2.zip
```

This bundle includes Step 16 approved local patch application plus wrap-up fixes for documentation drift and safer multi-file patch preparation.

## Current completed implementation

Completed:

- Phase 0 specs and project skeleton
- Rust workspace skeleton
- core Gadget/Pack manifest types
- `.gadgets/` init
- append-only audit ledger with hash chaining
- evidence bundle writer with artifact hashes
- deterministic policy engine
- observe-only Filesystem Read Gadget
- deterministic mock provider and Coordinator stub
- config loading and provider profile selection
- installed pack and Gadget manifest loading
- pack validation
- OpenAI provider adapter
- Anthropic provider adapter
- Patch Writer plan-only mode
- approval request/record scaffolding
- approved local patch application through `gadgets patch apply`

Current supported local flow:

```bash
gadgets init
gadgets ask "Propose a patch to update the docs"
gadgets approval request-patch <plan-run-id>
gadgets approval approve <approval-request-id> <approver>
gadgets approval verify <approval-request-id>
gadgets patch apply <approval-request-id>
gadgets evidence verify <apply-run-id>
gadgets ledger verify
```

## Current progress estimate

- Core safety spine through approved local patch application: 100% at checkpoint/code level.
- Local Developer MVP: about 75-80% complete.
- Full Gadgets Framework roadmap: about 30-35% complete.

## Guardrails

Maintain these guardrails:

- Do not add arbitrary shell execution.
- Do not add a generic root-shell Gadget.
- Do not let provider SDK tool calls bypass the Gadgets runtime.
- Do not add Linux admin actions yet.
- Do not add database, cloud, deployment, or production actions yet.
- Do not run unconfigured commands.
- Do not allow model output to choose raw commands directly.
- Do not weaken evidence/audit requirements.
- Do not silently change manifests or policies without updating Markdown docs.
- Keep comments and docs ASCII-only.
- Keep Step 17 separate from Git/PR behavior.

## Important review findings to carry forward

### Historical validation note superseded by Step 22

Cargo/Rust is not available in the ChatGPT sandbox. A Rust-enabled environment must run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

### Approval expiry needs a future contract

Approval records include `expires_at`, but strict format/enforcement is not fully locked. Do not depend on expiry for security until a strict timestamp format and enforcement tests are implemented.

### Step 16 patch application was hardened

Patch apply now prepares all file changes before writing any file. Preserve this behavior. Do not reintroduce partial multi-file apply risk.

## Step 17 objective

Implement an **allowlisted Test Runner**.

The Test Runner should run only commands explicitly configured in `.gadgets/config.yaml`. It must not provide arbitrary shell access.

## Proposed Step 17 scope

Add a narrow test provider that supports:

- `test.run` capability
- configured command names, not arbitrary user/model commands
- fixed working directory from config
- command vector or carefully constrained command string
- stdout capture
- stderr capture
- exit status capture
- timeout field if simple and safe
- evidence artifacts
- audit events

## Suggested config shape

Extend `.gadgets/config.yaml` with something like:

```yaml
test_commands:
  - name: cargo_test
    command: cargo
    args:
      - test
    working_dir: "."
  - name: npm_test
    command: npm
    args:
      - test
    working_dir: "."
```

Prefer command plus args over a free-form shell string.

## Suggested CLI command

Start with explicit CLI behavior before provider/Coordinator auto-routing:

```bash
gadgets test run [--project <path>] <command-name>
```

Later, `gadgets ask` can route to the Test Runner after patch apply, but Step 17 should first build the provider and CLI safely.

## Step 17 acceptance criteria

- Add `test.run` capability support where needed.
- Add Test Runner manifest updates if needed.
- Add config parsing for allowlisted test commands.
- Add a test command provider that does not invoke a shell.
- Reject unknown command names.
- Reject commands not configured in `.gadgets/config.yaml`.
- Reject unsafe working directories outside the project boundary.
- Capture stdout, stderr, exit status, and duration if feasible.
- Create evidence artifacts, likely:
  - `test_command.txt`
  - `stdout.txt`
  - `stderr.txt`
  - `exit_status.txt`
  - `test_summary.md`
- Append audit events:
  - `run.started`
  - `action.allowed` or `action.denied`
  - `action.completed` or `action.failed`
  - `evidence.created`
  - `run.completed` or `run.failed`
- Do not apply patches.
- Do not run Git commands.
- Do not open PRs.
- Do not add arbitrary shell.
- Update Markdown docs.
- Update README/roadmap/implementation plan/specs.

## Recommended implementation sequence

1. Review the current bundle and confirm no drift before editing.
2. Add config structs for allowlisted test commands.
3. Add Test Runner provider module in `crates/gadgets-tools`.
4. Add CLI command `gadgets test run`.
5. Wire policy evaluation for `test.run`.
6. Write evidence bundle and audit events.
7. Update docs/specs.
8. Package a Step 17 checkpoint.

## Expected next checkpoint name

```text
gadgets-framework-step17-allowlisted-test-runner-v0_1.zip
```
