# Step 43 - Provider and Model Inventory Design

Date: 2026-05-13

## Goal

Step 43 adds a docs/spec-only design checkpoint for a provider and model inventory.

The inventory supports the Gadgets Framework AI RMF alignment work from Step 42, especially the Map and Govern functions. It identifies which model providers and model profiles are approved for a project, what data exposure labels they may receive, which runtime modes they may be used in, and what review evidence should exist before they are used for higher-risk work.

This step does not implement runtime enforcement. It defines the contract that a later read-only report or enforcement preview can consume.

## Why this matters

Gadgets Framework is LLM-agnostic. Provider adapters are integration surfaces, not security boundaries. A project may use a mock provider, OpenAI, Anthropic, local models, or future providers behind the same runtime authority boundary.

That flexibility creates a governance need: operators should be able to answer basic questions without reading source code or guessing from profile names.

Examples:

- Which providers are configured for this project?
- Which model profiles are approved for Safe, Team, or Production mode?
- Which data labels may be sent to each provider?
- Which provider credentials are referenced by environment variable name?
- Which tasks, packs, or Gadget families may use a model profile?
- Is a model profile approved for remote/network use?
- Is a provider disabled, deprecated, or pending review?
- What evidence supports the approval decision?

Step 43 defines those inventory concepts before adding runtime commands.

## Relationship to existing provider profiles

Current `.gadgets/config.yaml` provider profiles select an adapter and model:

```yaml
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
```

A provider/model inventory should not replace model profiles. Instead, it should add governance metadata around them.

Model profiles answer:

```text
How does the runtime call this provider profile?
```

The inventory answers:

```text
Why is this provider profile allowed, where may it be used, and what data may it receive?
```

## Inventory objects

### Provider record

A provider record describes the provider adapter and the high-level governance posture for that provider.

Recommended future fields:

```yaml
providers:
  - id: mock
    display_name: Deterministic Mock Provider
    adapter: mock
    status: enabled
    network_access: false
    credential_env: ""
    approved_modes:
      - safe
      - team
      - production
    allowed_data_labels:
      - public
      - internal
    retention_notes: local deterministic provider; no external retention
    review_owner: ""
    review_status: approved
```

Provider status values:

```text
enabled
disabled
deprecated
pending_review
```

Provider review status values:

```text
not_reviewed
approved
approved_with_conditions
denied
expired
```

### Model profile record

A model profile record maps an existing runtime `model_profiles` entry to governance controls.

Recommended future fields:

```yaml
model_profile_inventory:
  - profile: mock_default
    provider_id: mock
    model: deterministic-mock
    status: enabled
    purpose: local deterministic testing
    approved_modes:
      - safe
      - team
      - production
    allowed_task_kinds:
      - repo.inspect
    allowed_packs:
      - developer
    allowed_gadgets:
      - coordinator
      - filesystem.read
    allowed_data_labels:
      - public
      - internal
    denied_data_labels:
      - sensitive
      - secret
      - secret_prohibited
    requires_human_review_for_sensitive: true
    review_status: approved
```

The inventory should reference the existing model profile name. It should not duplicate secrets or API keys.

### Data exposure rule

A data exposure rule defines which information may be sent to providers.

Recommended labels:

```text
public
internal
sensitive
secret
secret_prohibited
```

Default provider exposure posture:

- `public` may be sent to approved providers when policy allows.
- `internal` may be sent to approved providers when policy allows.
- `sensitive` requires explicit policy approval before provider exposure.
- `secret` must not be sent to providers.
- `secret_prohibited` must not be sent to providers or written to provider-facing prompts.

The label `secret_prohibited` is intended for patterns such as API keys, private keys, credentials, tokens, signing material, and secret-bearing config values.

## Proposed future config shape

Step 43 does not implement this config. It reserves a possible future shape:

```yaml
ai_risk:
  provider_model_inventory:
    enabled: true
    require_inventory_for_live_providers: true
    require_inventory_for_team_mode: true
    require_inventory_for_production_mode: true
    default_data_label: internal

    providers:
      - id: mock
        display_name: Deterministic Mock Provider
        adapter: mock
        status: enabled
        network_access: false
        credential_env: ""
        approved_modes: [safe, team, production]
        allowed_data_labels: [public, internal]
        retention_notes: local deterministic provider; no external retention
        review_owner: ""
        review_status: approved

      - id: openai
        display_name: OpenAI
        adapter: openai
        status: pending_review
        network_access: true
        credential_env: OPENAI_API_KEY
        approved_modes: [safe]
        allowed_data_labels: [public, internal]
        denied_data_labels: [sensitive, secret, secret_prohibited]
        retention_notes: document project-specific retention assumptions before Team or Production use
        review_owner: ""
        review_status: not_reviewed

    model_profiles:
      - profile: mock_default
        provider_id: mock
        model: deterministic-mock
        status: enabled
        purpose: deterministic local testing
        approved_modes: [safe, team, production]
        allowed_task_kinds: [repo.inspect]
        allowed_packs: [developer]
        allowed_data_labels: [public, internal]
        denied_data_labels: [sensitive, secret, secret_prohibited]
        review_status: approved
```

## Future CLI shapes

Step 43 does not implement these commands. They are reserved as possible future targets:

```text
gadgets risk inventory providers [--project <path>] [--format text|json]
gadgets risk inventory models [--project <path>] [--format text|json]
gadgets risk inventory provider-model [--project <path>] [--format text|json]
gadgets risk inventory exposure [--project <path>] [--format text|json]
```

A later command should be read-only first. It should not call providers, mutate config, install packs, or change runtime enforcement.

## Future evidence artifacts

A future provider/model inventory report may create evidence artifacts such as:

```text
provider_model_inventory.yaml
provider_inventory_summary.md
model_profile_inventory_summary.md
data_exposure_summary.yaml
provider_review_findings.txt
missing_inventory_findings.txt
live_provider_findings.txt
provider_mode_approval_matrix.txt
```

Evidence must not include API key values, token values, private keys, signing material, or raw secret-bearing config.

Environment variable names such as `OPENAI_API_KEY` may be recorded because they identify configuration shape, not secret values.

## Future audit events

Future provider/model inventory events may include:

```text
ai.risk.provider_inventory.reviewed
ai.risk.model_inventory.reviewed
ai.risk.provider_inventory.warning
ai.risk.provider_inventory.missing
ai.risk.data_exposure.warning
ai.risk.data_exposure.denied
ai.risk.live_provider.review_required
```

## Future enforcement path

The recommended migration path is:

1. docs/spec inventory design
2. read-only inventory report
3. evidence-backed inventory report
4. warning-only runtime exposure checks
5. dry-run-deny exposure checks in Team and Production
6. hard-deny only after explicit review and validation

This mirrors the pack trust migration pattern. Hard-deny should not be the first implementation.

## Non-goals for Step 43

Step 43 does not add:

- runtime code
- new CLI commands
- provider calls
- provider disablement
- data exposure enforcement
- compliance certification claims
- provider credential storage
- secret scanning beyond existing redaction discussion
- hard-deny pack-load enforcement
- signing tools
- trust-root mutation
- pack registry behavior
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior
- broader Git behavior
- provider-side tool execution

## Recommended next step

Run external Rust validation before adding more runtime source changes. The validation pass should cover the source changes introduced in Steps 37 through 41.

After validation results are reviewed, the next bounded design step can be Step 44 docs/spec design for data exposure labels and provider prompt boundaries.
