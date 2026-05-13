# Local Repo Basic Example

This example demonstrates the current local Developer Pack workflow:

```bash
gadgets ask "Review this repo and explain how it is structured."
gadgets test run cargo_test
gadgets git status
gadgets git branch create feature/example
```

Expected behavior:

- repository inspection is observe-only
- patch planning is evidence-only until explicitly approved and applied
- test execution runs only named commands configured in `.gadgets/config.yaml`
- Git status uses one fixed local observe command
- Git branch creation uses one fixed local branch command after branch-name and protected-branch checks
- no arbitrary shell
- no remote Git operations
- guarded remote PR creation is disabled by default and requires explicit config
- denied secret paths
- evidence bundles written
- audit ledger updated
