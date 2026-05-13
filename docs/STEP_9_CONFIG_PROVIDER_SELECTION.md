# Step 9 - Config Loading and Provider Profile Selection

Date: 2026-05-12

## Purpose

Step 9 moves provider selection out of the hardcoded Coordinator flow and into local project configuration.

The runtime now loads `.gadgets/config.yaml`, validates the configured runtime mode and model profile, selects the configured provider profile, and then instantiates the provider adapter.

At Step 9, the deterministic mock provider was the only executable provider. As of Step 13, OpenAI and Anthropic are also available as opt-in live provider profiles behind the same runtime-controlled provider contract.

## Implemented

- Added `crates/gadgets-cli/src/config.rs`.
- Added config parsing and validation for:
  - schema version
  - runtime mode
  - model profile names
  - default model profile selection
  - provider/model presence
  - unsupported provider rejection
- Updated generated `.gadgets/config.yaml` to include:

```yaml
default_model_profile: mock_default
```

- Updated `gadgets ask` to load `.gadgets/config.yaml` before running.
- Updated `gadgets ask` to support:

```bash
gadgets ask [--project <path>] <request>
```

- Updated the mock provider wiring so provider and model names come from the selected profile.
- Updated `FilesystemReadRequest` so policy evaluation uses the configured runtime mode instead of a hardcoded Safe Mode value.

## Current supported providers

As of Step 13, these provider profile names are supported:

- `mock`
- `openai`
- `anthropic`

Mock remains the default and live providers are opt-in.

## Safety behavior preserved

The provider profile can influence which model adapter is selected, but it still cannot authorize or execute actions.

The existing authority path remains unchanged:

```text
Config selects provider profile
  -> provider emits structured handoff
    -> CLI validates handoff
      -> policy engine evaluates every action
        -> Filesystem Read provider reads only allowed files
          -> evidence bundle is written
            -> audit ledger records the run
```

## Not implemented yet

- provider cost/token budgets.
- provider streaming.
- filesystem writes.
- shell execution.
- patching.
- test running.
- Git/PR behavior.
- Linux admin actions.

## Acceptance criteria

Step 9 is complete when:

- `gadgets ask` loads `.gadgets/config.yaml`.
- missing config produces a clear `gadgets init` guidance message.
- `mock_default` is selected by default.
- provider/model names are reflected in the Coordinator output.
- unsupported providers are rejected before any action is attempted.
- policy/evidence/audit behavior remains enforced by the runtime.
