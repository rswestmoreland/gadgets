# Step 23 - Developer MVP Alpha Packaging

Date: 2026-05-13

## Purpose

Step 23 packages the validated Developer MVP as an understandable alpha milestone.

This step is documentation and example focused. It does not add runtime behavior, provider behavior, policy scope, Git behavior, PR behavior, Linux admin behavior, database behavior, cloud behavior, deployment behavior, or arbitrary shell execution.

## Baseline

Step 23 builds on the externally validated Step 22 baseline:

```text
gadgets-main.zip
validated commit: c5fbd78
validation status: passed end-to-end
```

Validation passed:

```text
cargo fmt --check                                      PASS
cargo check                                           PASS
cargo test                                            PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo build --release                                 PASS
```

## Work completed

- Added `docs/DEVELOPER_MVP_ALPHA.md`.
- Documented what the alpha can do today.
- Documented what the alpha intentionally cannot do.
- Added a concise safety model summary.
- Added a sample local `.gadgets/config.yaml` shape.
- Added a sample `test_commands` config.
- Added a disabled-by-default remote PR config example.
- Added a complete alpha workflow from inspection through optional guarded GitHub PR creation.
- Added evidence and audit explanation.
- Added troubleshooting notes.
- Added known limitations.
- Updated README to point to the alpha guide and move the next recommended step to Remote PR safety hardening.
- Updated roadmap and implementation plan to mark Step 23 complete.
- Updated the local example README to reference the alpha guide.
- Regenerated `FILE_MANIFEST.txt`.

## Boundaries preserved

Still not implemented:

- arbitrary shell
- generic root-shell Gadget
- provider-side tool execution bypass
- model-selected raw commands
- Git push, fetch, pull, merge, or rebase
- Git checkout or switch
- remote branch creation
- GitLab or Bitbucket PR/MR support
- Linux server administration behavior
- database behavior
- cloud behavior
- deployment behavior
- full secret scanner or DLP model
- pack signing and trust roots
- Team Mode approval workflows

## Result

The Developer MVP now has an alpha guide that explains the current usable workflow, safe defaults, configuration examples, evidence/audit model, troubleshooting paths, known limitations, and next hardening priorities.
