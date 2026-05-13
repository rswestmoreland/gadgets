# Step 8 - Mock Provider and Coordinator Stub

Date: 2026-05-12

## Purpose

Step 8 adds the first provider-facing Coordinator flow without introducing a live model provider or new authority.

The mock provider creates a deterministic structured handoff from the Coordinator to the Filesystem Read Gadget. The runtime still performs every meaningful safety check through policy, evidence, and audit.

## Implemented

### `crates/gadgets-provider`

Added:

- `ModelProvider` trait
- `ProviderRequest`
- `ProviderResponse`
- `ProviderResponseStatus`
- `ProviderError`
- deterministic `MockProvider`

The mock provider returns one structured handoff for the first vertical slice:

```text
coordinator -> filesystem.read
```

Task kind:

```text
repo.inspect
```

Target zone:

```text
local_repo
```

The mock provider does not:

- read files
- execute tools
- approve work
- mutate state
- call OpenAI
- call Anthropic
- bypass runtime policy

### `crates/gadgets-cli`

Updated `gadgets ask <request>` to:

1. create a run id
2. call the deterministic mock provider
3. validate the returned handoff targets `filesystem.read`
4. print the Coordinator plan and safety notes
5. pass handoff metadata to the Filesystem Read slice
6. keep filesystem action authorization inside the policy engine

### `crates/gadgets-tools`

Updated the Filesystem Read request and evidence output with Coordinator/handoff metadata.

The audit ledger now records additional events for the observe-only run:

- `run.started`
- `provider.response`
- `handoff.requested`
- `handoff.allowed`
- `action.allowed` or `action.denied`
- `evidence.created`
- `run.completed`

The evidence bundle can include a `coordinator_plan.md` artifact.

## Safety boundary preserved

The provider and Coordinator can only request a handoff.

They cannot directly perform `file.read`, `file.search`, or any future tool action. The Filesystem Read Gadget still sends every candidate directory and file through the deterministic policy engine.

## Current `ask` behavior

```bash
gadgets ask "Review this repo and explain how it is structured."
```

Current behavior:

- mock provider creates Coordinator handoff
- Filesystem Read Gadget inspects allowed paths
- denied paths are recorded
- evidence bundle is written
- audit ledger is appended
- no files are modified
- no shell commands are executed
- no live model provider is called

## Not implemented yet

- live OpenAI provider
- live Anthropic provider
- provider credentials
- model streaming
- real natural-language planning beyond the deterministic mock handoff
- patch planning
- filesystem writes
- approval persistence
- test runner
- Git/PR behavior
- Linux admin behavior
