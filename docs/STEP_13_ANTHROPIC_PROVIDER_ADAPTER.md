# Step 13 - Anthropic Provider Adapter

Date: 2026-05-12

## Purpose

Step 13 adds an Anthropic provider adapter behind the same `ModelProvider` trait used by the mock and OpenAI providers.

The adapter preserves the core Gadgets authority rule:

Models may reason, propose, summarize, and request actions. Only the Gadgets runtime may authorize and execute actions.

The Anthropic adapter is intentionally limited to Coordinator planning. It can request a structured handoff, but it cannot execute tools, read files, approve work, or mutate state.

## What changed

### Provider crate

`crates/gadgets-provider` now includes:

- `AnthropicProvider`
- `DEFAULT_ANTHROPIC_ENDPOINT`
- `DEFAULT_ANTHROPIC_API_KEY_ENV`
- `DEFAULT_ANTHROPIC_VERSION`
- Anthropic Messages API request construction
- Claude structured output request through `output_config.format`
- response `content[].text` extraction
- refusal and max-token stop handling
- structured output parsing into runtime `HandoffRequest` values
- target Gadget validation against `allowed_target_gadgets`

### CLI config

`crates/gadgets-cli/src/config.rs` now accepts provider profiles with:

```yaml
model_profiles:
  anthropic_default:
    provider: anthropic
    model: claude-sonnet-4-6
    api_key_env: ANTHROPIC_API_KEY
    endpoint: https://api.anthropic.com/v1/messages
```

`api_key_env` and `endpoint` are optional. If omitted, the Anthropic adapter uses:

- `ANTHROPIC_API_KEY`
- `https://api.anthropic.com/v1/messages`
- `anthropic-version: 2023-06-01`

### Init defaults

`gadgets init` still defaults to the deterministic mock provider.

It now writes commented OpenAI and Anthropic profile examples into `.gadgets/config.yaml` so users can opt into either live provider without changing the safety model.

## Runtime behavior

When `.gadgets/config.yaml` selects an Anthropic provider profile:

1. `gadgets ask` loads config.
2. The CLI creates `AnthropicProvider` from the selected model profile.
3. The adapter sends a Coordinator-only request to the Anthropic Messages API.
4. Claude is asked for structured JSON matching the Coordinator handoff schema.
5. The adapter parses the structured handoff request.
6. The CLI verifies that the handoff is to `filesystem.read` and uses `repo.inspect`.
7. The existing Filesystem Read slice runs through policy, evidence, and audit.

The provider is not allowed to execute tools directly.

## Safety notes

The Anthropic adapter does not enable:

- filesystem writes
- shell execution
- patching
- test running
- Git/PR behavior
- Linux admin behavior
- database/cloud/deployment behavior
- provider-side tool execution

Provider output is treated as an untrusted structured request.

The runtime still checks:

- installed pack
- loaded Gadget manifest
- handoff target
- task kind
- Gadget capability
- tool allowlist
- runtime mode
- filesystem path policy
- denied paths
- evidence generation
- audit events

## Configuration example

```yaml
schema_version: gadgets.framework/config/v0.1

mode: safe
default_model_profile: anthropic_default

model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock

  anthropic_default:
    provider: anthropic
    model: claude-sonnet-4-6
    api_key_env: ANTHROPIC_API_KEY

installed_packs:
  - developer
```

Then run:

```bash
export ANTHROPIC_API_KEY="..."
gadgets ask "Review this repo and explain how it is structured."
```

## Reference notes

The adapter uses the Anthropic Messages API endpoint and `x-api-key` / `anthropic-version` headers. It requests structured output using the Claude API `output_config.format` JSON schema shape.

## Validation status

Static review, ASCII scan, path-length scan, YAML sanity checks, and ZIP integrity checks were performed in this environment.

Rust tests are included but were not executed in this sandbox.

## Next recommended step

Return to local workflow functionality with Patch Writer plan-only mode.

The next slice should let a Gadget propose a diff as evidence without writing files.
