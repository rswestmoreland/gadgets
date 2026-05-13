# Step 3 - Init Project State

Date: 2026-05-12

## Scope

This step adds the first CLI behavior:

```bash
gadgets init [path]
```

The command creates local `.gadgets/` project state with Safe Mode defaults.

## Created layout

```text
.gadgets/
  config.yaml
  README.md
  .gitignore
  packs/
  gadgets/
  zones/
  runs/
  ledger/
  evidence/
  approvals/
```

## Default behavior

The generated `config.yaml` uses:

- Safe Mode.
- mock provider.
- deterministic mock model profile.
- Developer Pack selected.
- local repository zone.
- denied secret/protected paths.
- approval required for file writes.
- no allowlisted test commands yet.
- default protected Git branch list configured.

## Git behavior

The generated `.gadgets/.gitignore` ignores volatile local runtime state:

```text
runs/
ledger/
evidence/
approvals/
```

The generated config and local README are not ignored by default.

## Safety posture

This step does not add:

- provider calls
- filesystem inspection
- audit persistence
- evidence persistence
- shell execution
- patch application
- Linux admin actions

It only creates local project state.

## Acceptance criteria

- `gadgets init` creates the expected directories and files.
- `gadgets init` is idempotent.
- generated config uses Safe Mode.
- generated config uses the mock provider.
- generated config requires approval for file writes.
- generated config denies common secret/protected paths.
- generated config includes default protected Git branches.
