# Step 20 - Local Developer MVP Hardening Checkpoint

Date: 2026-05-13

## Summary

Step 20 hardened the local Developer MVP after Steps 17 through 19 introduced test execution, local Git status, protected local branch creation, approved local commit scaffolding, and local PR body generation.

No remote behavior was added.

## Code changes

- Locked approval expiration to strict UTC RFC3339 without fractional seconds.
- Added expiration format validation when creating patch approval requests.
- Added expiration enforcement before approval records can be created.
- Added expiration enforcement during approval verification.
- Added tests for strict timestamp parsing, invalid expiration format, future expiration, expired approval recording, and expired verification.
- Fixed an approval helper return bug found during the Step 20 review.
- Updated CLI help for `--expires-at`.

## Documentation changes

- Added Step 20 hardening documentation.
- Added the Local Developer MVP walkthrough.
- Updated README, roadmap, implementation plan, open decisions, approval spec, and example config.
- Updated the file manifest.

## Boundaries preserved

Step 20 does not add:

- remote PR creation was not implemented in this checkpoint; Step 21 later added guarded remote PR creation
- Git push, pull, fetch, merge, or rebase
- arbitrary shell
- provider-side tool execution
- Linux admin behavior
- database, cloud, or deployment behavior

## Validation performed in this environment

- Static review of changed files.
- ASCII scan.
- YAML parse check for YAML files.
- Path-length scan.
- ZIP integrity check.

Historical checkpoint note: Rust validation was not run at this step. Superseded by Step 22: full Rust validation passed at commit c5fbd78.

## External validation required

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Remaining known gaps

- External Rust validation is still required.
- Evidence failure after mutation needs future hardening.
- Redaction is basic and not a full secret scanning system.
- Remote PR creation was not implemented in this checkpoint; Step 21 later added guarded remote PR creation.
- Git push/pull/fetch/merge/rebase are not implemented.
- Linux admin, database, cloud, and deployment packs remain future work.
- Pack signing and trust roots are not implemented.

## Recommended next step

Historical checkpoint note: Step 22 later completed this validation flow at commit c5fbd78.
