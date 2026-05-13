# Provider Adapter Specification

Provider adapters allow the runtime to use different model vendors without making the vendor SDK the security boundary.

## Local config selection

Provider selection is configured through `.gadgets/config.yaml`.

```yaml
default_model_profile: mock_default

model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
```

The mock provider remains the default. OpenAI and Anthropic are available as opt-in live provider profiles behind the same provider contract.

## Adapter responsibilities

- message formatting
- model selection
- structured output handling
- streaming translation
- token/cost metadata
- provider error normalization

## Adapter non-responsibilities

Adapters do not decide:

- whether a tool may execute
- whether a handoff is allowed
- whether approval is required
- whether a zone boundary is valid


## Step 8 mock provider baseline

The first provider implementation is deterministic and local. It returns a structured handoff from `coordinator` to `filesystem.read` for `repo.inspect` tasks.

The mock provider is intentionally not an authority boundary. It cannot execute tools, read files, approve actions, mutate state, or call a live model vendor.

Runtime policy must validate every action requested after the handoff.


## OpenAI adapter baseline

The OpenAI provider adapter uses the Responses API and bearer-token authentication through `OPENAI_API_KEY` by default. It requests strict structured output for Coordinator handoff planning. The adapter parses the model response into `ProviderResponse` and `HandoffRequest` values, but the runtime still validates the handoff and controls every tool/action provider call.

OpenAI provider profile example:

```yaml
model_profiles:
  openai_default:
    provider: openai
    model: gpt-5.5
    api_key_env: OPENAI_API_KEY
```


## Anthropic adapter baseline

The Anthropic provider adapter uses the Messages API and `x-api-key` authentication through `ANTHROPIC_API_KEY` by default. It requests structured Coordinator handoff planning with `output_config.format` using a JSON schema. The adapter parses the model response into `ProviderResponse` and `HandoffRequest` values, but the runtime still validates the handoff and controls every tool/action provider call.

Anthropic provider profile example:

```yaml
model_profiles:
  anthropic_default:
    provider: anthropic
    model: claude-sonnet-4-6
    api_key_env: ANTHROPIC_API_KEY
```

Provider adapters must not expose raw tool execution APIs to models.

## Patch Writer handoff

Providers may propose a `patch.writer` handoff with `task_kind: repo.patch.plan` for patch/change/test/doc/fix requests. Runtime must still validate the handoff and then policy-check the `patch.plan` action before evidence is created. Providers must not claim that a patch was applied.
