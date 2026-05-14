# Provider and Model Inventory Specification

Date: 2026-05-13

## Purpose

This specification defines a future provider and model inventory for Gadgets Framework.

The inventory records which AI providers and model profiles are approved for a project, which runtime modes they may be used in, what data labels they may receive, and what review evidence supports that posture.

This specification does not implement runtime enforcement. It defines a stable contract for future reporting and later warning or dry-run enforcement.

## Principles

- Provider adapters are integration surfaces, not authority boundaries.
- Provider output remains untrusted.
- The Gadgets runtime authorizes actions.
- Inventory records must not contain secret values.
- Inventory records should be auditable and evidence-backed before enforcement.
- Live provider use should be more reviewable than deterministic local mock use.
- Data exposure should be explicit, label-based, and deny-by-default for secrets.

## Provider inventory

A provider inventory record describes a provider adapter and its project approval posture.

Recommended fields:

```yaml
id: openai
display_name: OpenAI
adapter: openai
status: pending_review
network_access: true
credential_env: OPENAI_API_KEY
api_base: https://api.openai.com
approved_modes:
  - safe
allowed_data_labels:
  - public
  - internal
denied_data_labels:
  - sensitive
  - secret
  - secret_prohibited
retention_notes: document project-specific retention assumptions before Team or Production use
review_owner: ""
review_status: not_reviewed
review_expires_at: ""
notes: ""
```

Allowed `status` values:

```text
enabled
disabled
deprecated
pending_review
```

Allowed `review_status` values:

```text
not_reviewed
approved
approved_with_conditions
denied
expired
```

## Model profile inventory

A model profile inventory record maps an existing `.gadgets/config.yaml` model profile to governance controls.

Recommended fields:

```yaml
profile: openai_default
provider_id: openai
model: gpt-5.5
status: pending_review
purpose: general reasoning through Coordinator only
approved_modes:
  - safe
allowed_task_kinds:
  - repo.inspect
  - repo.patch.plan
allowed_packs:
  - developer
allowed_gadgets:
  - coordinator
allowed_data_labels:
  - public
  - internal
denied_data_labels:
  - sensitive
  - secret
  - secret_prohibited
requires_human_review_for_sensitive: true
review_status: not_reviewed
review_expires_at: ""
notes: ""
```

The `profile` field must match a configured `model_profiles` entry. The inventory should not duplicate API keys or tokens.

## Data exposure labels

Recommended labels:

```text
public
internal
sensitive
secret
secret_prohibited
```

Default semantics:

| Label | Default provider exposure posture |
| --- | --- |
| `public` | Allowed when policy allows. |
| `internal` | Allowed when policy allows. |
| `sensitive` | Requires explicit policy approval before provider exposure. |
| `secret` | Must not be sent to providers. |
| `secret_prohibited` | Must not be sent to providers or written to provider-facing prompts. |

Secret-like values include API keys, bearer tokens, private keys, signing material, passwords, credentials, session cookies, and secret-bearing config values.

## Project inventory shape

Recommended future config shape:

```yaml
ai_risk:
  provider_model_inventory:
    enabled: true
    require_inventory_for_live_providers: true
    require_inventory_for_team_mode: true
    require_inventory_for_production_mode: true
    default_data_label: internal
    providers: []
    model_profiles: []
```

## Reporting posture

A future report should produce one of these posture values:

```text
not_configured
mock_only
live_provider_review_required
configured_with_warnings
ready_for_team_review
ready_for_production_dry_run_review
not_ready_blocking_findings
```

Suggested posture rules:

- `not_configured`: no inventory is present.
- `mock_only`: only deterministic local providers are configured.
- `live_provider_review_required`: live providers exist but lack inventory or review status.
- `configured_with_warnings`: inventory exists but contains non-blocking findings.
- `ready_for_team_review`: inventory is sufficient for Team Mode review.
- `ready_for_production_dry_run_review`: inventory is sufficient for Production dry-run review.
- `not_ready_blocking_findings`: secrets, missing reviews, or denied providers are present.

## Evidence artifacts

Future provider/model inventory reports may create:

```text
provider_model_inventory.yaml
provider_inventory_summary.md
model_profile_inventory_summary.md
data_exposure_summary.yaml
provider_mode_approval_matrix.txt
provider_review_findings.txt
missing_inventory_findings.txt
live_provider_findings.txt
```

Evidence must not include secret values.

## Audit events

Future audit events may include:

```text
ai.risk.provider_inventory.reviewed
ai.risk.model_inventory.reviewed
ai.risk.provider_inventory.warning
ai.risk.provider_inventory.missing
ai.risk.live_provider.review_required
ai.risk.data_exposure.warning
ai.risk.data_exposure.denied
```

## Non-goals

This specification does not add:

- runtime commands
- provider calls
- provider disablement
- credential storage
- secret scanning beyond the existing redaction roadmap
- hard-deny behavior
- compliance certification
- legal advice
- pack install or registry behavior
- provider-side tool execution
