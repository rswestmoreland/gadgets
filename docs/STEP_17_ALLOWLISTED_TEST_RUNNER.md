# Step 17 - Allowlisted Test Runner

Date: 2026-05-12

Status: implemented at checkpoint/code level.

## Goal

Implemented a narrow Test Runner that runs only named test commands configured in `.gadgets/config.yaml`.

First supported entrypoint:

```bash
gadgets test run [--project <path>] <test-command-name>
```

## Non-goals

Step 17 must not add:

- arbitrary shell
- model-supplied command execution
- free-form user-supplied command execution
- provider SDK tool execution
- patch application inside the Test Runner
- Git status, branch, commit, push, or PR behavior
- Linux admin behavior
- database, cloud, deployment, or production behavior

## Configuration contract

Supported config shape:

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

Rules:

- `name` must be non-empty.
- names must be unique.
- `command` must be non-empty.
- `working_dir` must be relative to the project root.
- `working_dir` must not contain parent traversal.
- `working_dir` must resolve inside the project boundary.
- `timeout_seconds` must be greater than zero when provided.
- command strings must come only from config.

## Execution contract

The Test Runner:

1. load `.gadgets/config.yaml`
2. find the named test command
3. reject unknown command names
4. reject unsafe working directories
5. load the Developer Pack and `test.runner` Gadget manifest
6. evaluate `test.run` through deterministic policy
7. run the configured command without `sh -c`
8. capture stdout
9. capture stderr
10. capture exit status
11. capture duration if practical
12. record pass/fail
13. write evidence artifacts
14. append audit events

## Evidence artifacts

Implemented artifacts:

```text
summary.md
bundle.yaml
test_command.txt
working_dir.txt
stdout.txt
stderr.txt
exit_status.txt
duration.txt
policy_decision.txt
assumptions.txt
```

A nonzero test exit status should be recorded with output. It should not be hidden as a successful run.

## Audit events

Implemented audit events:

```text
test.requested
policy.checked
action.allowed
action.denied
test.started
test.completed
test.failed
evidence.created
run.completed
```

## Safety notes

Allowlisting controls what command the Gadgets runtime launches. It does not sandbox the test process itself. Test commands may execute project code with local user permissions. This is why Step 17 must remain explicit, local, named, configured, audited, and evidence-producing.

stdout/stderr may contain sensitive data. The implementation does not send test output to model providers. It caps stdout/stderr evidence output and redacts secret-like output lines before evidence write.

## Acceptance checklist

- [x] `gadgets test run [--project <path>] <test-command-name>` exists.
- [x] A named allowlisted test command can run.
- [x] Unknown command names are rejected.
- [x] Empty command names are rejected.
- [x] Commands are loaded from `.gadgets/config.yaml`.
- [x] Arbitrary command strings are not accepted.
- [x] Command execution does not use `sh -c`.
- [x] Shell metacharacters are rejected or unsupported in configured commands.
- [x] `working_dir` is constrained to the project boundary.
- [x] Parent traversal is rejected.
- [x] Runtime policy checks `test.run` before execution.
- [x] stdout/stderr/exit status are captured.
- [x] pass/fail is recorded.
- [x] Evidence bundle is written.
- [x] Audit ledger events are appended.
- [x] Test failure is recorded without hiding output.
- [x] No patch application happens inside the Test Runner.
- [x] No Git/PR behavior is added.
- [x] No Linux admin behavior is added.
- [x] Docs and specs are updated.

## External validation commands

When a Rust toolchain is available, run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
