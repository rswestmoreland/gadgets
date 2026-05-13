You are continuing work on the Gadgets Framework project.

Start with REVIEW ONLY. Do not write code until the review is complete, drift is identified, and the exact Step 36 implementation/design plan is clear.

Current authoritative baseline:

gadgets-main(1).zip
validated commit: 14b0a4f

Treat this validated Step 35 baseline as authoritative. Do not let older Step 21, Step 22, Step 23, Step 24, Step 25, Step 26, Step 27, Step 28, Step 29, Step 30, Step 31, Step 32, Step 33, Step 34, or pre-validation Step 35 bundles override it.

Project summary:

Gadgets Framework is a safety-first, vendor-neutral framework for purpose-built AI workers called Gadgets. A Gadget is a least-privilege AI worker that operates inside fixed capability zones and collaborates only through policy-enforced handoffs. Models may reason, propose, summarize, and request actions. Only the Gadgets runtime may authorize and execute actions. Provider SDK behavior, model tool-calling, prompts, and agent handoff features are integration surfaces, not final security boundaries.

Current validated status:

The full Rust validation flow passed on commit 14b0a4f:

- cargo fmt --check
- cargo check
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings
- cargo build --release

Rust/Cargo versions used:

- rustc 1.89.0 (29483883e 2025-08-04)
- cargo 1.89.0 (c24e10642 2025-06-23)

Current implemented baseline:

- Core types and manifest parsing.
- gadgets init and local .gadgets project state.
- Append-only audit ledger with hash-chain verification.
- Evidence bundle writer with artifact hashes.
- Deterministic policy engine.
- Observe-only Filesystem Read Gadget wired through policy, evidence, and audit.
- Deterministic mock provider and Coordinator stub.
- Config loading and provider profile selection.
- Installed pack and Gadget manifest loading.
- gadgets pack validate.
- OpenAI provider adapter.
- Anthropic provider adapter.
- Patch Writer plan-only mode.
- Approval record scaffolding for local writes.
- Approved local patch application.
- Allowlisted Test Runner.
- Local Git status.
- Protected local branch creation.
- Approved local commit scaffolding.
- Local PR body generation.
- Local Developer MVP hardening with approval expiration enforcement.
- Guarded remote PR creation through GitHub API, disabled by default and gated by config.
- Remote PR dry-run mode, branch constraints, and duplicate-open-PR handling.
- Shared best-effort secret/output redaction helper.
- Pack trust/signing design.
- Non-enforcing pack trust check diagnostics.
- Non-mutating trust root inspection.
- Pack trust diagnostic evidence and audit emission.
- Pack trust policy preview.
- Signature metadata verification scaffold.
- Ed25519 signature verification diagnostics.
- Signature-aware pack trust policy preview.
- Dual license metadata: MIT OR Apache-2.0.

Step 36 goal:

Step 36 should be Pack Trust Enforcement Design and Dry-Run Gate Plan. Start docs-first. Do not implement runtime pack-load denial until the design is reviewed and approved.

Review scope before planning:

- README.md
- docs/ARCHITECTURE.md
- docs/ROADMAP.md
- docs/IMPLEMENTATION_PLAN.md
- docs/OPEN_DECISIONS.md
- docs/DECISION_RECORD.md
- specs/PACK_MODEL.md
- specs/PACK_TRUST_SIGNING_SPEC.md
- specs/AUDIT_LEDGER_SPEC.md
- specs/EVIDENCE_BUNDLE_SPEC.md
- crates/gadgets-cli/src/pack_trust.rs
- crates/gadgets-cli/src/main.rs
- crates/gadgets-cli/src/manifest_loader.rs
- crates/gadgets-policy/src/lib.rs
- pack manifests under packs/
- examples/local-repo-basic/.gadgets/config.yaml

Step 36 design requirements:

- Define exact Safe Mode behavior.
- Define exact Team Mode behavior.
- Define exact Production Mode behavior.
- Define whether enforcement is off, warn-only, dry-run-deny, or hard-deny in each mode.
- Define a migration path from diagnostics to enforcement.
- Define config switches and safe defaults.
- Define audit events for pack-load trust decisions.
- Define evidence artifacts for pack-load trust decisions.
- Define how built-in packs are treated.
- Define how project-local packs are treated.
- Define how unsigned local packs are treated.
- Define how invalid signatures are treated.
- Define how expired signatures or trust roots are treated.
- Define failure behavior if evidence or audit cannot be written.
- Define rollback/safe-mode behavior.
- Define test plan names only, unless coding is later approved.

Strict non-goals for Step 36 unless explicitly approved later:

- Do not add arbitrary shell.
- Do not add generic root-shell behavior.
- Do not let provider SDK tool calls bypass the Gadgets runtime.
- Do not weaken Safe Mode.
- Do not add Linux admin mutation.
- Do not add database behavior.
- Do not add cloud behavior.
- Do not add deployment behavior.
- Do not add Git push, pull, fetch, merge, or rebase.
- Do not add branch checkout or switch.
- Do not broaden remote PR creation.
- Do not add GitLab support.
- Do not add pack install or pack update behavior.
- Do not add registry downloads.
- Do not add signing tools.
- Do not mutate trust roots.
- Do not enforce pack loading until the enforcement design is approved.
- Do not expose secret values to providers or evidence.

After review, provide:

1. Summary of review findings.
2. Drift/gaps found.
3. Exact Step 36 design plan.
4. Files expected to change for docs-first Step 36.
5. Acceptance checklist.
6. Recommendation on whether Step 36 should remain docs-only or proceed to a narrow dry-run implementation in a later step.
