# Gadgets Framework - Updated Plan and Progress After Step 32

Date: 2026-05-13

## Current progress estimate

| Scope | Estimate | Notes |
|---|---:|---|
| Core safety spine | 100% | Implemented and externally validated through the Step 22 baseline. |
| Local Developer MVP | 95% | Alpha-packaged with local patch, test, Git, PR-body, guarded PR creation, and hardening docs. |
| Guarded remote PR MVP | 80-85% | GitHub PR creation exists and safety hardening is present; external validation is still deferred after later changes. |
| Pack trust diagnostics | 55-60% | Design, inspection, roots, evidence/audit, policy preview, and signature metadata verification scaffold are present. Real cryptographic verification and enforcement are still deferred. |
| Full Gadgets Framework roadmap | 52-56% | Developer workflow and pack trust diagnostics are strong; Team workflows, Linux Server Admin, database/cloud/deployment packs, signing enforcement, and broader integrations remain future work. |

## Step 32 completed

- [x] Added `gadgets pack trust signature [--project <path>] <pack>`.
- [x] Validates `pack.signature.yaml` required metadata fields.
- [x] Validates signature metadata version.
- [x] Validates `ed25519` algorithm metadata.
- [x] Validates strict UTC timestamp shape for `created_at` and `expires_at`.
- [x] Validates pack id/version references.
- [x] Validates manifest hash reference.
- [x] Validates content-manifest hash reference when `pack.contents.yaml` is present.
- [x] Checks local trust-root publisher/key/algorithm reference.
- [x] Checks that trust-root metadata allows the pack id.
- [x] Writes signature metadata diagnostic evidence.
- [x] Appends `pack.signature.checked` and `evidence.created` audit events.
- [x] Keeps command diagnostic-only and non-enforcing.

## Current pack trust command surface

```bash
gadgets pack trust check [--project <path>] <pack>
gadgets pack trust roots [--project <path>]
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
gadgets pack trust signature [--project <path>] <pack>
```

## Still not implemented

- [ ] Cryptographic signature verification
- [ ] Signing tools
- [ ] Trust-root mutation
- [ ] Pack trust enforcement
- [ ] Pack install/update behavior
- [ ] Registry downloads
- [ ] Team/Production trust enforcement
- [ ] Git push/fetch/pull/merge/rebase
- [ ] Arbitrary shell
- [ ] Linux admin/database/cloud/deployment behavior

## Validation note

The last full external Rust validation baseline remains the Step 22 baseline at commit `c5fbd78`. Steps 24, 25, 27, 28, 30, 31, and 32 include Rust source changes after that baseline. External Rust validation should be rerun before a release tag.

## Recommended next step

Proceed with Step 33: cryptographic verification design finalization, docs-first, before implementing real signature verification.
