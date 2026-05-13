# Phase 0 Baseline

Date: 2026-05-12

## Locked direction

- Rust core runtime.
- CLI-first MVP.
- Provider-neutral model adapters.
- Local `.gadgets/` project state.
- YAML manifests and config.
- JSONL audit/event streams.
- Markdown human-readable evidence summaries.
- Built-in deterministic policy checks first.
- Safe Mode default.
- Developer Pack first.
- Linux Server Admin Observe Pack before Change Pack.
- No generic root-shell Gadget.
- Approval required for file writes in v0.1; validated non-protected local Git branch ref creation is handled by its own narrow policy context.
- Evidence and audit required for meaningful work.
- Arbitrary shell deferred except allowlisted test commands.

## First vertical slice

Observe-only repository inspection.

```bash
gadgets ask "Review this repo and explain how it is structured."
```

## First implementation goals

1. Create local project state with `gadgets init`.
2. Load runtime config.
3. Load Developer Pack manifests.
4. Load Filesystem Read Gadget manifest.
5. Run mock Coordinator flow.
6. Authorize file reads through policy.
7. Deny secret/protected paths.
8. Produce evidence bundle.
9. Append audit events.
10. Verify audit hash chain.

## Explicit deferrals

- Production deployments.
- Database writes.
- Cloud mutation.
- Firewall changes.
- Package installation/removal.
- Data/log deletion.
- Service restart.
- Reboot.
- Secrets rotation.
- Public pack registry.
- Arbitrary shell execution.
