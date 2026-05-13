# Step 16 - Approved Local Patch Application

Date: 2026-05-12

## Purpose

Step 16 closes the local patch loop created by Step 14 and Step 15.

Step 14 produces a plan-only `proposed.patch` artifact. Step 15 creates an approval request and approval record bound to the exact patch SHA-256 and scope hash. Step 20 adds expiration enforcement. Patch apply uses that approval verification and applies the patch only when the approval record, expiration, and patch artifact still verify.

## Implemented behavior

Step 16 adds approved local patch application through:

```bash
gadgets patch apply [--project <path>] <approval-request-id>
```

The command:

1. Loads `.gadgets/config.yaml`.
2. Verifies the Developer Pack is installed.
3. Loads the Patch Writer Gadget manifest through the manifest loader.
4. Verifies the approval request, approval record, and expiration.
5. Verifies the exact `proposed.patch` SHA-256 still matches the approval scope.
6. Parses the approved unified diff.
7. Evaluates every target path through the deterministic policy engine with approval present.
8. Applies only approved, path-scoped local file changes.
9. Records before/after file hashes.
10. Writes a new evidence bundle for the apply run.
11. Appends audit events for approval use, policy decisions, file writes, evidence, and completion.

## New crate behavior

`crates/gadgets-tools/src/patch_apply.rs` adds:

- `PatchApplyRequest`
- `PatchApplyReport`
- `PatchApplyError`
- `run_patch_apply()`

The provider supports a deliberately narrow subset of unified diff behavior for the first local developer MVP.

Supported:

- regular text file additions/updates
- relative repository paths
- path-scoped policy checks
- before/after SHA-256 evidence

Rejected:

- absolute paths
- parent traversal
- deletion patches to `/dev/null`
- unsafe paths
- context mismatches
- paths outside writable boundaries
- denied paths such as `.git/`, `.gadgets/`, `.env`, `secrets/`, private keys, or secret-like paths

## Approval hardening

Step 16 also tightens approval verification.

`verify_approval()` now requires both:

- `request.yaml`
- `approval.yaml`

A request-only approval directory is no longer considered valid for apply. This prevents a future apply flow from treating a scoped request as if it were an actual human approval.

## Evidence artifacts

The apply run writes a new evidence bundle under:

```text
.gadgets/runs/<apply-run-id>/evidence/
```

New artifacts include:

- `summary.md`
- `bundle.yaml`
- `applied.patch`
- `files_changed.txt`
- `before_after_hashes.txt`
- `approval_verification.txt`
- `policy_decisions.txt`
- `assumptions.txt`

## Audit events

Step 16 appends audit events such as:

- `run.started`
- `approval.used`
- `action.allowed`
- `action.completed`
- `evidence.created`
- `run.completed`

Denied or invalid approval paths append failure-oriented events where possible before stopping.

## Review hardening added at session close

A wrap-up review tightened the Step 16 apply path so multi-file patches are prepared before any file is written. The provider now verifies policy for every target, parses the full supported unified diff, computes all before/after contents and hashes, and only then writes files. This reduces the chance of partial application when a later hunk fails to match the working tree.

## Safety boundary

Step 16 still does not implement:

- shell execution
- test running
- Git commands
- staging or committing files
- PR creation
- provider-side tool execution
- Linux admin actions
- database actions
- cloud actions
- deployment actions

The only new mutation is local file patch application after exact approval and policy verification.

## Intended workflow

```bash
gadgets ask "Propose a patch to update docs"
gadgets approval request-patch <plan-run-id>
gadgets approval approve <approval-request-id> richard@example.com
gadgets approval verify <approval-request-id>
gadgets patch apply <approval-request-id>
gadgets evidence verify <apply-run-id>
gadgets ledger verify
```

## Next recommended step

Step 17 now implements an allowlisted Test Runner.

The Test Runner remains separate from patch application. It only runs commands explicitly configured in `.gadgets/config.yaml`, captures stdout/stderr/exit status/duration as evidence, and appends audit events.

## Step 20 update

Patch apply now inherits strict approval expiration enforcement through `verify_approval()`. Expired approvals are rejected before patch application can proceed.
