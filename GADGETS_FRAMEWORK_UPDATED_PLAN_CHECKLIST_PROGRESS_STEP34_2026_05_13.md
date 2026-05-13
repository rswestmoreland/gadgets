# Gadgets Framework - Updated Plan and Progress After Step 34

Date: 2026-05-13

## Current status

Step 34 is complete at checkpoint level. It adds diagnostic-only Ed25519 verification to `gadgets pack trust signature`.

The last full external Rust validation baseline remains commit `c5fbd78`. External validation has not been rerun after Steps 24, 25, 27, 28, 30, 31, 32, and 34.

## Progress estimate

| Scope | Estimate | Notes |
|---|---:|---|
| Core safety spine through guarded remote PR creation | 100% | Implemented and previously validated through commit `c5fbd78`. |
| Local Developer MVP | 98-99% | Alpha-packaged; later changes need revalidation. |
| Guarded remote PR MVP | 85-90% | Disabled by default, dry-run by default, branch-constrained, duplicate-aware, evidence/audit-backed. |
| Pack trust diagnostics | 65-70% | Design, inspection, trust-root diagnostics, evidence/audit emission, policy preview, metadata diagnostics, and Ed25519 verification diagnostics are implemented. Enforcement and signing tools remain deferred. |
| Full Gadgets Framework roadmap | 52-56% | Developer workflow and non-enforcing trust diagnostics are advanced; Team/Linux/database/cloud/deployment packs remain future work. |

## Step 34 completed

- [x] Added Ed25519 verification dependency.
- [x] Added base64 decoding dependency.
- [x] Built deterministic `gadgets-pack-signature-v1` payload.
- [x] Verified raw-byte hashes for `pack.yaml` and `pack.contents.yaml`.
- [x] Verified content manifest entries and hashes.
- [x] Rejected unsafe, duplicate, unsorted, missing, symlink, and non-file content entries.
- [x] Required `pack.yaml` in signed contents.
- [x] Rejected `pack.signature.yaml` in signed contents.
- [x] Matched publisher, key id, algorithm, and allowed pack id against trust roots.
- [x] Checked signature and trust-root expiration metadata.
- [x] Verified Ed25519 signatures when metadata and trust-root keys are available.
- [x] Wrote verification evidence artifacts.
- [x] Preserved non-enforcing behavior.

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

Step 35 - Pack trust policy preview with real signature results.

The next step should update `gadgets pack trust preview` to consume the real signature diagnostic result while remaining non-enforcing. This will make Safe/Team/Production previews reflect actual `trusted_signed`, expired, mismatch, and metadata-invalid outcomes before any enforcement is added.
