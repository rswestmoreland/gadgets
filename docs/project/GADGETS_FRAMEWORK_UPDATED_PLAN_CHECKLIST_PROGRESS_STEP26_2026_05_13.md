# Gadgets Framework - Updated Plan and Progress After Step 26

Date: 2026-05-13

## Current baseline

Current checkpoint: `gadgets-framework-step26-pack-trust-signing-design-v0_1.zip`

Last full external Rust validation baseline remains Step 22 commit `c5fbd78`.

Steps 24 and 25 included Rust source changes after that validation. Step 26 is documentation/specification only. External Rust validation should still be rerun before a release tag.

## Progress summary

| Scope | Estimate | Notes |
|---|---:|---|
| Core safety spine | 100% | Implemented and externally validated through Step 22. |
| Local Developer MVP | 95% | Implemented, alpha-packaged, and hardened. Remaining work is polish and validation after later source changes. |
| Guarded remote PR MVP | 85-90% | GitHub PR creation exists, disabled by default, dry-run by default, branch-constrained, duplicate-aware, and evidence/audit-backed. |
| Pack trust/signing | 20-25% | Design contract is locked; enforcement, signing tools, and trust-root commands are not implemented. |
| Full Gadgets Framework roadmap | 46-50% | Developer workflow is strong and pack trust is designed. Team workflows, Linux admin packs, database/cloud/deployment packs, pack trust enforcement, and broader integrations remain future work. |

## Completed through Step 26

- [x] Core types and manifest parsing
- [x] Local `.gadgets/` project state
- [x] Audit ledger and hash-chain verification
- [x] Evidence bundles and artifact hashing
- [x] Deterministic policy engine
- [x] Filesystem Read observe-only flow
- [x] Mock provider and Coordinator stub
- [x] Config loading and provider profiles
- [x] Pack/Gadget manifest loading
- [x] Pack validation
- [x] OpenAI provider adapter
- [x] Anthropic provider adapter
- [x] Patch Writer plan-only mode
- [x] Approval records
- [x] Approved local patch application
- [x] Allowlisted Test Runner
- [x] Local Git status
- [x] Protected local branch creation
- [x] Approved local commit scaffolding
- [x] Local PR body generation
- [x] Approval expiration enforcement
- [x] Guarded remote PR creation
- [x] External Rust validation through Step 22 baseline
- [x] Developer MVP alpha packaging
- [x] Remote PR safety hardening
- [x] Shared best-effort redaction helper
- [x] Pack trust/signing design

## Step 26 completed checklist

- [x] Defined pack identity.
- [x] Defined content manifest shape.
- [x] Defined detached signature record shape.
- [x] Defined local trust roots.
- [x] Defined verification failure behavior.
- [x] Defined Safe mode unsigned local behavior.
- [x] Defined Team/Production Mode requirements.
- [x] Defined audit/evidence expectations for future trust checks.
- [x] Added `specs/PACK_TRUST_SIGNING_SPEC.md`.
- [x] Updated roadmap, implementation plan, architecture, open decisions, decision record, and pack model.

## Still not implemented

- [ ] Signature verification code
- [ ] Pack signing tools
- [ ] Pack trust enforcement
- [ ] Trust-root management commands
- [ ] Registry downloads
- [ ] Pack install/update behavior
- [ ] Team/Production trust enforcement
- [ ] Git push/fetch/pull/merge/rebase
- [ ] Git checkout/switch
- [ ] GitLab support
- [ ] Linux admin behavior
- [ ] Database/cloud/deployment behavior
- [ ] Full DLP or complete secret scanner

## Recommended next step

Proceed with Step 27 - Pack trust inspection scaffold.

Step 27 should add a non-enforcing command:

```text
gadgets pack trust check [--project <path>] <pack>
```

The first implementation should inspect and report trust status only. It should not enforce signatures, add signing tools, download packs, or mutate trust roots.

## Validation note

Do not claim a new Rust validation baseline until the external flow is rerun:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
