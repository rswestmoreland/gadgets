You are continuing work on the Gadgets Framework project.

Start with REVIEW ONLY. Do not add new feature scope until the review and validation status are clear.

Current authoritative checkpoint:

gadgets-framework-step21-license-metadata-v0_1.zip

Treat this checkpoint as authoritative. Do not let older Step 16, Step 17, Step 18, Step 19, Step 20, Step 21, or skeleton bundles override this license metadata checkpoint.

Project summary:

Gadgets Framework is a safety-first, vendor-neutral framework for purpose-built AI workers called Gadgets. A Gadget is a least-privilege AI worker that operates inside fixed capability zones and collaborates only through policy-enforced handoffs. Models may reason, propose, summarize, and request actions. Only the Gadgets runtime may authorize and execute actions. Provider SDK behavior, model tool-calling, prompts, and agent handoff features are integration surfaces, not final security boundaries.

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
- Dual license metadata: MIT OR Apache-2.0.

License and author metadata now locked:

- License: MIT OR Apache-2.0
- Author: Richard S. Westmoreland <dev@rswestmore.land>
- Copyright: Copyright 2026 Richard S. Westmoreland

Review goals:

1. Verify the license files are present and coherent:
   - LICENSE.md
   - LICENSE-MIT
   - LICENSE-APACHE
   - NOTICE
   - AUTHORS.md
   - COPYRIGHT.md
2. Verify Cargo metadata:
   - root Cargo.toml uses license = "MIT OR Apache-2.0"
   - every crate Cargo.toml uses license = "MIT OR Apache-2.0"
   - author metadata uses Richard S. Westmoreland <dev@rswestmore.land>
   - no old unlicensed metadata string remains
3. Verify docs:
   - README.md has license and author details
   - docs/OPEN_DECISIONS.md marks license closed
   - docs/DECISION_RECORD.md records the license decision
   - docs/ROADMAP.md no longer lists license decision as open
4. Verify FILE_MANIFEST.txt matches the repository.
5. Review current Step 21 remote PR creation behavior for drift, but do not expand feature scope.
6. Run the Rust validation flow.

Required validation commands:

cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release

If validation fails:

- Make bounded fixes only.
- Prefer the smallest changes needed to pass validation.
- Do not add arbitrary shell.
- Do not add generic root-shell behavior.
- Do not weaken Safe Mode.
- Do not let provider SDK tool calls bypass runtime policy.
- Do not add Linux admin mutation.
- Do not add database, cloud, deployment, or production behavior.
- Do not add Git push, pull, fetch, merge, or rebase unless explicitly requested in a later step.
- Do not broaden remote PR creation beyond the config-gated GitHub PR create path already present.
- Do not expose secret values to providers or evidence.
- Keep documentation and comments ASCII-only.

After validation and bounded fixes, provide:

1. Summary of review findings.
2. Exact validation commands run and pass/fail results.
3. List of files changed.
4. Any drift or safety issues found.
5. What was fixed.
6. What remains not implemented.
7. Updated checkpoint zip.
8. Updated plan/checklist/progress note.

Expected non-goals for this validation step:

- No new provider.
- No remote push.
- No branch checkout or switch.
- No Git fetch, pull, merge, or rebase.
- No GitLab support.
- No Linux admin behavior.
- No database/cloud/deployment behavior.
- No arbitrary shell.
- No production mode expansion.
