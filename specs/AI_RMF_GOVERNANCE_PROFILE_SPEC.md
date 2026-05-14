# AI RMF Governance Profile Specification

Date: 2026-05-13

## Purpose

This specification defines a future-facing governance profile for aligning Gadgets Framework runtime controls with the NIST AI Risk Management Framework operating model.

This specification is not a compliance claim. It is an engineering map that helps operators understand which Gadgets controls can support AI risk governance, mapping, measurement, and management.

## Reference model

The NIST AI RMF Core is organized around four functions:

- Govern
- Map
- Measure
- Manage

Gadgets uses those functions as an organizing model for runtime control evidence. The framework remains LLM-agnostic and provider-neutral.

## Control families

### Govern controls

Govern controls define who is allowed to configure, approve, review, and operate AI-assisted work.

Current supporting controls:

- runtime mode
- policy engine
- approval records
- pack trust configuration
- decision records
- audit ledger
- evidence bundle requirements

Future profile fields:

```yaml
governance:
  owner: ""
  approvers: []
  risk_tolerance: low
  review_cadence: per_release
  production_hard_deny_approved: false
```

### Map controls

Map controls identify the AI-assisted workflow, its providers, data exposure, capability zones, packs, and dependencies.

Current supporting controls:

- provider profiles
- installed packs
- pack manifests
- Gadget manifests
- capability declarations
- zone declarations
- effective source classification

Future profile fields:

```yaml
inventory:
  providers: []
  models: []
  packs: []
  capabilities: []
  zones: []
  data_labels: []
```

### Measure controls

Measure controls collect repeatable evidence about whether runtime guardrails are operating as expected.

Current supporting controls:

- audit ledger verification
- evidence bundle verification
- policy decision outputs
- pack trust gate warning and dry-run denial events
- signature diagnostics
- test results
- redaction helper behavior

Future profile fields:

```yaml
measurement:
  require_policy_outcome_metrics: true
  require_trust_gate_metrics: true
  require_redaction_metrics: true
  require_audit_health: true
  require_evidence_completeness: true
```

### Manage controls

Manage controls define how the project responds to AI risk findings.

Current supporting controls:

- Safe Mode
- approval-required mutation
- disabled-by-default remote PR behavior
- pack-load dry-run gate
- future hard-deny state
- fail-closed evidence and audit requirements

Future profile fields:

```yaml
management:
  require_rollback_guidance: true
  require_incident_events: true
  require_provider_disablement: true
  require_safe_mode_recovery: true
```

## Data exposure labels

Future data exposure labels should be simple, explicit, and safe by default:

```text
public
internal
sensitive
secret
secret_prohibited
```

Default rule:

- `public` and `internal` may be sent to configured providers when policy allows.
- `sensitive` requires explicit policy approval before provider exposure.
- `secret` and `secret_prohibited` must not be sent to providers or written into provider-facing prompts.

## Incident classes

Future AI risk incident classes should include:

```text
provider_unsafe_output
provider_policy_bypass_attempt
malformed_handoff
secret_exposure_attempt
redaction_failure
pack_trust_failure
audit_write_failure
evidence_write_failure
approval_scope_violation
unexpected_provider_behavior
```

## Report posture values

Future AI risk reporting should use stable posture values:

```text
not_configured
collecting_evidence
review_required
ready_for_team_mode
ready_for_production_dry_run
candidate_for_hard_deny_review
not_ready_blocking_findings
```

## Non-goals

This specification does not add:

- runtime commands
- new action authority
- provider-side execution
- compliance certification
- legal advice
- hard-deny enforcement
- signing tools
- trust-root mutation
- pack registry behavior

## Step 43 provider/model inventory addition

Step 43 adds `specs/PROVIDER_MODEL_INVENTORY_SPEC.md` as the detailed inventory contract for provider and model mapping.

The provider/model inventory supports:

- Govern: provider review status, owner, approved modes, and review expiration.
- Map: provider IDs, model profiles, allowed packs, allowed Gadgets, task kinds, and data labels.
- Measure: future inventory findings, provider review warnings, missing inventory findings, and evidence-backed reports.
- Manage: provider disablement planning, sensitive-data exposure review, and future warning/dry-run enforcement paths.

The inventory complements existing `model_profiles`. It must not store API key values, tokens, private keys, signing material, or raw secret-bearing config values.
