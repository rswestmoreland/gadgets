# Local Repo Basic Example

This example demonstrates the current local Developer Pack workflow. For the complete alpha guide, see `../../docs/DEVELOPER_MVP_ALPHA.md`.

Common commands:

```bash
gadgets ask "Review this repo and explain how it is structured."
gadgets test run cargo_test
gadgets git status
gadgets git branch create feature/example
gadgets git pr body <approval-request-id> --test-run <run-id> --commit-run <run-id>
```

Expected behavior:

- repository inspection is observe-only
- patch planning is evidence-only until explicitly approved and applied
- test execution runs only named commands configured in `.gadgets/config.yaml`
- Git status uses one fixed local observe command
- Git branch creation uses one fixed local branch command after branch-name and protected-branch checks
- no arbitrary shell
- no Git push, fetch, pull, merge, rebase, checkout, or switch
- guarded remote PR creation is disabled by default, dry-run by default, and requires explicit config
- denied secret paths
- evidence bundles written
- audit ledger updated
