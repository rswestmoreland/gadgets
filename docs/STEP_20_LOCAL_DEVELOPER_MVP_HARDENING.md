# Step 20 - Local Developer MVP Validation and Hardening

Date: 2026-05-13

## Goal

Step 20 stabilizes the local Developer MVP after the Step 17 through Step 19 execution surfaces were added.

The focus is validation readiness and safety hardening, not new remote capability.

Step 20 does not add:

- remote PR creation was not implemented in this checkpoint; Step 21 later added guarded remote PR creation
- Git push, pull, fetch, merge, or rebase
- arbitrary shell
- provider-side tool execution
- Linux admin behavior
- database, cloud, or deployment behavior

## Scope

Step 20 performs bounded hardening across the existing local workflow:

1. Review the Step 19 checkpoint for drift.
2. Preserve the local-only workflow boundaries.
3. Lock approval expiration format and enforcement.
4. Document the full local Developer MVP smoke workflow.
5. Reconcile README, roadmap, implementation plan, specs, and examples.
6. Record external validation commands because Cargo is unavailable in this environment.

## Approval expiration contract

Approval expiration is now locked to a strict UTC timestamp shape:

```text
YYYY-MM-DDTHH:MM:SSZ
```

Example:

```text
2999-01-01T00:00:00Z
```

Notes:

- Fractional seconds are not accepted.
- Time zone offsets such as `+00:00` are not accepted in the current MVP.
- Leap seconds are not accepted.
- Invalid calendar dates are rejected.
- Expiration is enforced before approval recording and during approval verification.

This keeps the first contract deterministic without adding a time parsing crate.

## Enforcement points

Expiration is checked in the approval crate:

- `create_patch_approval_request()` validates the optional expiration format.
- `approve_request()` rejects already expired approval requests.
- `verify_approval()` reports expired or malformed expiration values as invalid verification.

Because patch apply, approved local commit, and PR body generation already call `verify_approval()`, expired approvals are rejected before those workflows can use the approval.

## Local Developer MVP smoke workflow

A complete local workflow now follows this shape:

```bash
gadgets init
gadgets ask "Propose a patch..."
gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z
gadgets approval approve <approval-request-id> <approver>
gadgets approval verify <approval-request-id>
gadgets patch apply <approval-request-id>
gadgets test run <test-command-name>
gadgets git status
gadgets git branch create <branch-name>
gadgets git commit approved-patch <approval-request-id>
gadgets git pr body <approval-request-id> --test-run <run-id> --commit-run <run-id>
gadgets evidence verify <run-id>
gadgets ledger verify
```

`<test-command-name>` must exist in `.gadgets/config.yaml` under `test_commands`.

## Evidence and audit consistency

The local MVP has separate evidence bundles for:

- Filesystem Read observe runs
- Patch Writer plan-only runs
- Patch apply runs
- Test Runner runs
- Git status runs
- Git branch creation runs
- Git commit runs
- Local PR body generation runs

Meaningful actions append audit events to `.gadgets/ledger/events.jsonl`.

## Output and secret handling

The local MVP includes bounded, basic secret-like output redaction in the command-capturing providers added in Steps 17 and 18.

Current limitation:

- Redaction is intentionally modest.
- It should not be treated as a full DLP or secret scanner.
- Future hardening should centralize and expand redaction rules before team or production use.

## Validation status

Historical checkpoint note: Rust validation was not run at this step. Superseded by Step 22: full Rust validation passed at commit c5fbd78.

Run externally:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Acceptance checklist

- [x] Approval expiration format locked.
- [x] Invalid expiration format rejected when creating approval requests.
- [x] Expired approvals rejected when recording approval.
- [x] Expired approvals rejected by verification.
- [x] Patch apply continues to use approval verification.
- [x] Approved local commit continues to use approval verification.
- [x] PR body generation continues to use approval verification.
- [x] Local Developer MVP walkthrough documented.
- [x] README, roadmap, implementation plan, specs, and example config reconciled.
- [x] No remote PR creation was added in Step 20; Step 21 later added guarded remote PR creation.
- [x] No Git push, pull, fetch, merge, or rebase added.
- [x] No arbitrary shell added.
- [x] No Linux admin, database, cloud, or deployment behavior added.
- [ ] External Rust validation still required.

## Recommended next step

Historical checkpoint note: Step 22 later completed external Rust validation and bounded fixes at commit c5fbd78.

After validation passes, the next design step should be either:

1. Developer MVP release packaging and polish, or
2. guarded remote PR creation behind explicit configuration and additional approval gates. Step 21 later implemented this narrow path.
