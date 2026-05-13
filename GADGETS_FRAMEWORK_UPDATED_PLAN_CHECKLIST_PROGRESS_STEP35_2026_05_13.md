# Gadgets Framework - Updated Plan and Progress After Step 35

Date: 2026-05-13

## Current status

Step 35 is complete at checkpoint level. It updates the non-enforcing `gadgets pack trust preview` command to consume real signature diagnostic results from Step 34.

The last full external Rust validation baseline remains commit `c5fbd78`. External validation has not been rerun after Steps 24, 25, 27, 28, 30, 31, 32, 34, and 35.

## Progress estimate

| Scope | Estimate | Notes |
|---|---:|---|
| Core safety spine through guarded remote PR creation | 100% | Implemented and previously validated through commit `c5fbd78`. |
| Local Developer MVP | 98-99% | Alpha-packaged; later changes need revalidation. |
| Guarded remote PR MVP | 85-90% | Disabled by default, dry-run by default, branch-constrained, duplicate-aware, evidence/audit-backed. |
| Pack trust diagnostics | 75-80% | Design, inspection, trust-root diagnostics, evidence/audit emission, policy preview, metadata diagnostics, Ed25519 verification diagnostics, and signature-aware policy preview are implemented. Enforcement and signing tools remain deferred. |
| Full Gadgets Framework roadmap | 54-58% | Developer workflow and non-enforcing trust diagnostics are advanced; Team/Linux/database/cloud/deployment packs remain future work. |

## Step 35 completed

- [x] Updated `gadgets pack trust preview` to consume signature diagnostic results.
- [x] Added signature metadata decision to preview reports.
- [x] Added signature presence to preview reports.
- [x] Added cryptographic verification performed/valid status to preview reports.
- [x] Added content manifest valid status to preview reports.
- [x] Added signature and trust-root expiration status to preview reports.
- [x] Updated Safe Mode preview to report signature findings while still allowing project-local development packs.
- [x] Updated Team Mode preview to allow only valid trusted signatures diagnostically.
- [x] Updated Production Mode preview to allow only valid trusted signatures diagnostically.
- [x] Added `signature_policy_inputs.txt` evidence.
- [x] Preserved diagnostic-only audit behavior.
- [x] Fixed stale Step 27 wording and a malformed duplicated `.to_string()` line found in `pack_trust.rs`.

## Still deferred

- [ ] Pack trust enforcement.
- [ ] Signing tools.
- [ ] Trust-root mutation.
- [ ] Pack install/update behavior.
- [ ] Registry downloads.
- [ ] Team/Production pack-load enforcement.
- [ ] Git push/fetch/pull/merge/rebase.
- [ ] Arbitrary shell.
- [ ] Linux admin/database/cloud/deployment behavior.

## Recommended next step

Step 36 - Pack trust enforcement design and dry-run gate plan.

The next step should be docs-first and should define the exact Team/Production enforcement gate before any runtime pack-load denial is implemented. It should include the future CLI/config switches, audit events, evidence artifacts, and migration path from diagnostic preview to enforcement.
