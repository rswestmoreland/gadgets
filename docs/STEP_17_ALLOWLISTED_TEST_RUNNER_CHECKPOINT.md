# Step 17 - Allowlisted Test Runner Checkpoint

Date: 2026-05-12

## Status

Step 17 is implemented at checkpoint/code level.

The Test Runner adds one explicit local CLI path:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The command name must match an entry in `.gadgets/config.yaml`. The user prompt and model output cannot supply raw command strings.

## Implemented behavior

- Added `test_commands` to runtime configuration.
- Added validation for test command names, duplicate names, empty commands, invalid working directories, parent traversal, absolute working directories, and zero-second timeouts.
- Added `gadgets test run [--project <path>] <test-command-name>`.
- Added an allowlisted Test Runner provider in `crates/gadgets-tools`.
- Added deterministic policy support for `test.run` with an explicit allowlisted-test context.
- Loaded the existing `test.runner` Gadget manifest from the Developer Pack.
- Executed configured commands with `std::process::Command` instead of `sh -c`.
- Rejected shell metacharacters and unsupported quoting in configured commands.
- Constrained `working_dir` to a project-relative directory that resolves inside the project root.
- Captured stdout, stderr, exit code, duration, pass/fail status, and timeout status.
- Capped captured output before evidence write.
- Redacted secret-like output lines before evidence write.
- Wrote evidence bundles for test runs.
- Appended audit events for request, policy, allow/deny, start, completion/failure, evidence creation, and run completion.

## Configuration shape

Test commands are disabled by default:

```yaml
test_commands: []
```

A local project can opt in by adding named entries:

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

The CLI receives only the name, for example:

```bash
gadgets test run cargo_test
```

## Evidence artifacts

Each test run writes a separate evidence bundle under `.gadgets/runs/<run-id>/evidence/`.

Artifacts include:

- `summary.md`
- `bundle.yaml`
- `assumptions.txt`
- `test_command.txt`
- `stdout.txt`
- `stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `working_dir.txt`
- `policy_decision.txt`

## Audit events

The provider appends audit events for:

- `test.requested`
- `policy.checked`
- `action.allowed`, `action.denied`, or `action.requires_approval`
- `test.started`
- `test.completed` or `test.failed`
- `evidence.created`
- `run.completed`

A nonzero test exit is recorded as a failed test result with stdout/stderr evidence. It is not hidden as a successful run.

## Boundaries preserved

Step 17 does not add:

- arbitrary shell
- `sh -c`
- model-supplied command strings
- user-supplied raw command strings
- provider-side tool execution
- patch application inside the Test Runner
- Git status, branch, commit, push, or PR behavior
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior

## Safety notes

Allowlisting controls which local command is launched. It does not create a full OS sandbox around the launched test process. The project should continue treating test execution as a local Developer Pack feature and avoid enabling risky commands in `.gadgets/config.yaml`.

Captured stdout and stderr are capped and secret-like lines are redacted before evidence write. This is a first-pass safety measure, not a complete secret scanner.

## Files changed for Step 17

Code:

- `crates/gadgets-cli/src/config.rs`
- `crates/gadgets-cli/src/init.rs`
- `crates/gadgets-cli/src/main.rs`
- `crates/gadgets-cli/src/manifest_loader.rs`
- `crates/gadgets-policy/src/lib.rs`
- `crates/gadgets-tools/src/lib.rs`
- `crates/gadgets-tools/src/test_runner.rs`

Docs/specs/examples:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/OPEN_DECISIONS.md`
- `docs/ROADMAP.md`
- `docs/STEP_16_APPROVED_PATCH_APPLY.md`
- `docs/STEP_17_ALLOWLISTED_TEST_RUNNER.md`
- `docs/STEP_17_DOCS_ONLY_CHECKPOINT.md`
- `docs/STEP_17_ALLOWLISTED_TEST_RUNNER_CHECKPOINT.md`
- `specs/CAPABILITY_MODEL.md`
- `specs/EVIDENCE_BUNDLE_SPEC.md`
- `specs/PACK_MODEL.md`
- `specs/TOOL_ACTION_PROVIDER_SPEC.md`
- `examples/local-repo-basic/.gadgets/config.yaml`
- `FILE_MANIFEST.txt`

## Validation status

Cargo, rustc, and rustfmt were not available in this sandbox. No Rust validation was run here.

Run these externally before treating the checkpoint as validated:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Recommended next step

Proceed to Step 18: Git status/branch/commit scaffolding.

Keep Step 18 narrow. It should start with observe-only Git status and branch/commit scaffolding, preserve approval and evidence boundaries, and avoid remote push or PR creation by default.
