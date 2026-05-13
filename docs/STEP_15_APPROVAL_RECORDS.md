# Step 15 - Approval Record Scaffolding

Date: 2026-05-12

## Purpose

Step 15 adds local approval records for future local write operations.

This step does not apply patches, modify files, run tests, execute shell commands, stage Git changes, open pull requests, or perform Linux administration actions.

The goal is to create the safety gate that a later Patch Writer apply step must satisfy.

## Implemented behavior

A new `gadgets-approval` crate can:

- create a patch approval request for a plan-only Patch Writer run
- compute the SHA-256 hash of `.gadgets/runs/<run-id>/evidence/proposed.patch`
- bind the approval request to the exact patch hash
- compute a deterministic scope hash
- write `.gadgets/approvals/<approval-request-id>/request.yaml`
- record a local approval in `.gadgets/approvals/<approval-request-id>/approval.yaml`
- verify that the patch artifact still matches the approved hash
- verify that the approval record scope hash still matches the request
- reject path traversal identifiers
- refuse to overwrite existing approval artifacts

## New commands

```bash
gadgets approval request-patch <run-id>
gadgets approval approve <approval-request-id> <approver>
gadgets approval show <approval-request-id>
gadgets approval verify <approval-request-id>
gadgets approval id-for-run <run-id>
```

Each command also supports `--project <path>` where applicable.

## Approval request shape

Patch approval requests are stored under:

```text
.gadgets/approvals/<approval-request-id>/request.yaml
```

The request binds:

- action kind: `repo.patch.apply`
- executor Gadget: `patch.writer`
- zone: `local_repo`
- run ID
- evidence bundle ID
- proposed patch artifact path
- proposed patch SHA-256
- scope hash
- conditions
- optional expiry

## Approval record shape

Approvals are stored under:

```text
.gadgets/approvals/<approval-request-id>/approval.yaml
```

The approval record includes:

- approval ID
- approval request ID
- approver
- status
- scope hash
- approval timestamp
- optional expiry
- conditions copied from the request

## Scope hash

The scope hash is calculated from a deterministic canonical string containing:

- action kind
- executor Gadget
- zone
- run ID
- patch artifact path
- patch SHA-256

If the patch artifact changes, verification fails.

If the request is edited in a way that changes scope, verification fails.

## Audit behavior

The CLI appends audit events for:

- `approval.requested`
- `approval.approved`

Audit events are appended to the existing local ledger.

## Safety boundary

An approval record is not an execution grant by itself.

Approval-backed execution steps must still:

- load the approval record
- verify the scope hash
- verify the patch hash
- confirm approval is not expired
- evaluate policy
- ensure every modified path is allowed
- write evidence
- append audit events

## Deferred

- multi-action approval records beyond patch approval
- multi-approver quorum
- approval revocation
- approval UI
- Team Mode approval workflows
- production approval workflows

## Step 20 update

Approval expiry enforcement is now implemented for the local MVP. Expiration uses strict UTC RFC3339 without fractional seconds and is checked during approval request creation, approval recording, and approval verification.
