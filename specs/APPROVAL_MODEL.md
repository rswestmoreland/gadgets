# Approval Model

Approvals are scoped, optionally time-bound, evidence-linked records.

Approval authorizes one specific action plan, not general authority.

## Requires approval by default

- file writes in v0.1
- production deployment
- database migration
- firewall changes
- package install/remove
- data deletion
- service restart
- reboot
- secret rotation
- IAM changes

If the plan changes, approval is invalid.

## Step 15 local approval scaffolding

Local patch approvals are stored under `.gadgets/approvals/<approval-request-id>/`.

For plan-only Patch Writer runs, `gadgets approval request-patch <run-id> [--expires-at <RFC3339-UTC>]` creates a request that binds future local patch application to the exact SHA-256 hash of `.gadgets/runs/<run-id>/evidence/proposed.patch` and optional strict UTC expiration.

`gadgets approval approve <approval-request-id> <approver>` records a local approval for that exact scope hash. It does not apply the patch.

A later apply or approval-backed Git/PR-body step must still verify:

- request exists
- approval exists
- approval expiration is valid and not expired when present
- patch hash still matches
- scope hash still matches
- runtime policy allows every modified path
- required evidence and audit events can be produced



## Step 16 approval enforcement

A patch apply action requires both an approval request and an approval record. The approval record must match the request scope hash, the approval must not be expired, and the approved patch artifact hash must still match before any file write occurs.

## Step 20 approval expiration enforcement

Approval expiration is locked to strict UTC RFC3339 without fractional seconds:

```text
YYYY-MM-DDTHH:MM:SSZ
```

Example:

```text
2999-01-01T00:00:00Z
```

The current MVP rejects fractional seconds, time zone offsets, leap seconds, and malformed dates. Already-expired requests are rejected before approval recording and during verification.

Enforcement points:

- approval request creation validates the optional expiration format
- approval recording rejects expired requests
- approval verification rejects expired or malformed expiration metadata
- patch apply, approved local commit, and local PR body generation use approval verification before using the approval

