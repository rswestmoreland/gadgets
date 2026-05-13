# Gadgets Framework - Updated Plan and Progress After Step 21

Date: 2026-05-13

## Progress summary

- Core safety spine through guarded remote PR creation: implemented at checkpoint/code level.
- Local Developer MVP: historical estimate. Step 22 later completed external Rust validation at commit c5fbd78; remaining work is alpha packaging and polish.
- Full multi-pack roadmap: about 35-40% complete.

## Step 21 completed

- [x] Add guarded remote PR creation command.
- [x] Keep remote PR creation disabled by default.
- [x] Add `git.remote_pr` config.
- [x] Support GitHub provider only in this checkpoint.
- [x] Require verified approval request and approval record.
- [x] Enforce approval expiration through existing approval verification.
- [x] Require completed local PR body evidence.
- [x] Validate head and base branches.
- [x] Reject same head/base branch.
- [x] Load token only from configured environment variable.
- [x] Do not write token value to evidence.
- [x] Create one pull request through GitHub API.
- [x] Write remote PR evidence bundle.
- [x] Append audit events.
- [x] Update README, docs, specs, roadmap, config comments, example config, and file manifest.

## Still not implemented

- [ ] External Rust validation in this environment.
- [ ] Git push.
- [ ] Git fetch, pull, merge, or rebase.
- [ ] Checkout or switch.
- [ ] GitLab support.
- [ ] Fork-style PR head refs.
- [ ] Arbitrary shell.
- [ ] Model-provider tool execution.
- [ ] Linux admin behavior.
- [ ] Database/cloud/deployment behavior.
- [ ] Full secret scanner/redaction model.
- [ ] Pack signing/trust roots.

## Next recommended step

Historical checkpoint note: Step 22 later completed external Rust validation and bounded fixes at commit c5fbd78:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```
