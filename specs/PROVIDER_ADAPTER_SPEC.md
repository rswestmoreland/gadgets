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

## Provider and model inventory design

Step 43 defines a future provider/model inventory layer in `specs/PROVIDER_MODEL_INVENTORY_SPEC.md`.

The inventory does not replace provider profiles. Provider profiles define how the runtime calls an adapter and model. The inventory defines why that provider/profile is approved, where it may be used, what data labels it may receive, and what review evidence supports that use.

Provider adapters must continue to follow these rules:

- They must not execute tools directly.
- They must not decide approval, policy, or zone outcomes.
- They must not receive secret values unless a future policy explicitly allows a safe non-secret representation.
- They must not expose provider SDK tool-calling as a runtime authority boundary.
- They must not treat inventory approval as permission to bypass Gadget policy.

Future provider/model inventory reports may record provider IDs, model profile names, model identifiers, adapter names, environment variable names, data exposure labels, review status, and retention notes. They must not record API key values, token values, private keys, signing material, or secret-bearing config.
