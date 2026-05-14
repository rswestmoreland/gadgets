# Step 36 Checkpoint - Pack Trust Enforcement Design and Dry-Run Gate Plan

Date: 2026-05-13

Status: complete as a docs-first design checkpoint.

## Summary

Step 36 defines the future pack-load trust enforcement model without adding runtime enforcement code.

The step locks:

- enforcement states: `off`, `warn-only`, `dry-run-deny`, and `hard-deny`
- exact Safe, Team, and Production behavior
- effective source classification for built-in, project-local, and mixed-source packs
- treatment for unsigned local packs
- treatment for invalid, mismatched, expired, or unknown signatures
- future config shape and safe defaults
- audit event vocabulary for dry-run and hard-deny pack-load decisions
- evidence artifacts for pack-load trust decisions
- failure behavior when evidence or audit cannot be written
- explicit rollback behavior
- future test plan names only

## Files changed

Docs/spec/config-example only:

- `docs/STEP_36_PACK_TRUST_ENFORCEMENT_DRY_RUN_GATE_PLAN.md`
- `docs/STEP_36_PACK_TRUST_ENFORCEMENT_DRY_RUN_GATE_PLAN_CHECKPOINT.md`
- `docs/ARCHITECTURE.md`
- `docs/ROADMAP.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/OPEN_DECISIONS.md`
- `docs/DECISION_RECORD.md`
- `specs/PACK_MODEL.md`
- `specs/PACK_TRUST_SIGNING_SPEC.md`
- `specs/AUDIT_LEDGER_SPEC.md`
- `specs/EVIDENCE_BUNDLE_SPEC.md`
- `examples/local-repo-basic/.gadgets/config.yaml`
- `docs/project/GADGETS_FRAMEWORK_UPDATED_PLAN_CHECKLIST_PROGRESS_STEP36_2026_05_13.md`
- `FILE_MANIFEST.txt`

## Explicit non-changes

Step 36 does not change Rust source code and does not implement enforcement.

No changes were made to:

- provider behavior
- runtime action execution
- pack loading behavior
- policy engine code
- manifest loader code
- pack trust CLI code
- trust-root mutation
- signing tools
- registry downloads
- pack install/update behavior
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior
- Git push, pull, fetch, merge, rebase, checkout, or switch

## Recommended next step

Step 37 should implement a narrow dry-run-only pack-load trust gate, if approved.

Step 37 should not add hard-deny enforcement first. It should evaluate effective pack source, emit evidence/audit for would-deny outcomes, and continue to allow loading so dry-run results can be reviewed.
