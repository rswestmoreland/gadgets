# Step 12 - OpenAI Provider Adapter

Date: 2026-05-12

## Purpose

Step 12 adds the first live model provider adapter while preserving the core Gadgets authority rule:

Models may reason, propose, summarize, and request actions. Only the Gadgets runtime may authorize and execute actions.

The OpenAI adapter is intentionally limited to Coordinator planning. It can request a structured handoff, but it cannot execute tools, read files, approve work, or mutate state.

## What changed

### Provider crate

`crates/gadgets-provider` now includes:

- `OpenAiProvider`
- `DEFAULT_OPENAI_ENDPOINT`
- `DEFAULT_OPENAI_API_KEY_ENV`
- OpenAI Responses API request construction
- strict JSON schema request for Coordinator handoff output
- response `output_text` extraction
- structured output parsing into runtime `HandoffRequest` values
- target Gadget validation against `allowed_target_gadgets`
- provider errors for missing API key, HTTP failures, invalid responses, and invalid structured output

### CLI config

`crates/gadgets-cli/src/config.rs` now accepts provider profiles with:

```yaml
model_profiles:
  openai_default:
    provider: openai
    model: gpt-5.5
    api_key_env: OPENAI_API_KEY
    endpoint: https://api.openai.com/v1/responses
```

`api_key_env` and `endpoint` are optional. If omitted, the OpenAI adapter uses:

- `OPENAI_API_KEY`
- `https://api.openai.com/v1/responses`

### Init defaults

`gadgets init` still defaults to the deterministic mock provider.

It now writes a commented OpenAI profile example into `.gadgets/config.yaml` so users can opt in without changing the safety model.

## Runtime behavior

When `.gadgets/config.yaml` selects an OpenAI provider profile:

1. `gadgets ask` loads config.
2. The CLI creates `OpenAiProvider` from the selected model profile.
3. The adapter sends a Coordinator-only request to the OpenAI Responses API.
4. The model is instructed to return strict structured JSON.
5. The adapter parses the structured handoff request.
6. The CLI verifies that the handoff is to `filesystem.read` and uses `repo.inspect`.
7. The existing Filesystem Read slice runs through policy, evidence, and audit.

The provider is not allowed to execute tools directly.

## Safety notes

The OpenAI adapter does not enable:

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
default_model_profile: openai_default

model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock

  openai_default:
    provider: openai
    model: gpt-5.5
    api_key_env: OPENAI_API_KEY

installed_packs:
  - developer
```

Then run:

```bash
export OPENAI_API_KEY="..."
gadgets ask "Review this repo and explain how it is structured."
```

## Reference notes

The adapter uses the OpenAI Responses API endpoint and bearer-token authentication. It requests structured output using the Responses API `text.format` JSON schema shape.

## Validation status

Static review, ASCII scan, path-length scan, YAML sanity checks, and ZIP integrity checks were performed in this environment.

Rust tests are included but were not executed in this sandbox.

## Next recommended step

Implement the Anthropic provider adapter behind the same provider trait, or pause provider work and start Patch Writer plan mode. The safer sequence is to add Anthropic next while provider contracts are fresh, then return to local plan-only Patch Writer behavior.
