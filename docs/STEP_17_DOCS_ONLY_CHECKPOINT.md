# Step 17 Documentation-Only Checkpoint

Date: 2026-05-12

## Scope

This checkpoint performed documentation-only cleanup before Step 17 implementation. It is now superseded by the Step 17 allowlisted Test Runner checkpoint.

No Rust source files, runtime manifests, provider code, policy code, patch application code, evidence code, or audit code were changed.

## Fixes made

- Updated active architecture documentation to reflect the current provider set: mock, OpenAI, and Anthropic are implemented.
- Removed stale wording for an unimplemented pack setup command from the active command surface.
- Updated roadmap status to remove stale Step 10 wording and reflect current Step 16 baseline.
- Added a Step 17 Test Runner boundary to active docs.
- Added a Step 17 planning document for the allowlisted Test Runner.
- Updated tool/action provider specification with the then-planned Test Runner contract.
- Updated evidence specification with planned test-run evidence artifacts.
- Updated capability specification with the `test.run` boundary.
- Updated pack model specification to describe the Test Runner as the next Developer Pack slice.
- Updated the example local `.gadgets/config.yaml` to match the current generated config shape, including `installed_packs`, `approval`, Anthropic example, and `test_commands: []`.
- Updated README with current command surface, Step 17 planned command shape, and current not-implemented list.

## Superseded items

The following items were still open at this documentation-only checkpoint, but are implemented in the Step 17 code checkpoint:

- Test Runner runtime implementation
- parsing and validation of `test_commands` in runtime config
- `gadgets test run`
- `test.run` policy execution path
- test-run evidence writer integration
- test-run audit events

The following items remain out of scope after Step 17:

- Git/PR behavior
- Linux admin behavior
- database/cloud/deployment behavior
