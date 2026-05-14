# Step 42 - AI RMF Alignment and Governance Profile Design

Date: 2026-05-13

## Goal

Step 42 adds a documentation-only design checkpoint for aligning the Gadgets Framework with the NIST AI Risk Management Framework operating model.

This step does not claim compliance or certification. It defines how Gadgets can expose governance, mapping, measurement, and management controls in a way that is useful for operators, security teams, compliance teams, and future integrations.

## Source references

NIST AI RMF 1.0 is a voluntary framework for managing risks associated with AI systems. The AI RMF Core is organized around four functions:

- Govern
- Map
- Measure
- Manage

NIST also publishes a Generative AI Profile for the AI RMF. That profile is relevant to Gadgets because Gadgets uses LLM providers as reasoning surfaces while keeping authority inside the runtime.

Reference links:

- https://www.nist.gov/itl/ai-risk-management-framework
- https://doi.org/10.6028/NIST.AI.100-1
- https://doi.org/10.6028/NIST.AI.600-1

## Why this belongs in Gadgets

Gadgets Framework is already designed around the idea that AI safety is not only a prompt-engineering problem. It is a runtime control problem involving:

- least privilege
- separation of duties
- explicit capability zones
- provider-neutral model adapters
- policy-enforced handoffs
- evidence bundles
- append-only audit records
- human approval gates
- pack trust and supply-chain diagnostics
- dry-run and hard-deny control states

These concepts map naturally to AI RMF-style governance and risk management without changing the runtime authority boundary.

## Alignment model

### Govern

Gadgets controls that support governance:

- runtime modes: Safe, Team, Production
- pack and Gadget manifests
- policy engine decisions
- approval records
- separation between model output and runtime authority
- pack trust diagnostics and dry-run enforcement posture
- decision records and project-level configuration

Future governance profile items:

- project AI governance profile
- role and responsibility inventory
- approval scope registry
- provider/model approval records
- explicit risk tolerance settings by project or team

### Map

Gadgets controls that support mapping:

- capability declarations
- zone declarations
- model provider profiles
- installed pack list
- effective pack source classification
- Gadget manifest source classification
- handoff records
- configured test commands and Git restrictions

Future mapping profile items:

- AI system inventory
- provider/model inventory
- data exposure inventory
- capability-to-risk mapping
- pack dependency and provenance map
- project-local override inventory

### Measure

Gadgets controls that support measurement:

- evidence bundles
- audit ledger verification
- pack trust warnings
- dry-run denial counts
- policy outcomes
- approval outcomes
- test results
- redaction behavior
- signature verification diagnostics

Future measurement profile items:

- policy outcome metrics
- redaction metrics
- provider refusal or unsafe-output metrics
- dry-run denial trend reporting
- evidence completeness checks
- audit-chain health summaries
- trust-root and signature freshness reporting

### Manage

Gadgets controls that support management:

- Safe Mode defaults
- approval gates for mutation
- runtime refusal on invalid approvals
- pack-load dry-run gates
- future hard-deny gate
- rollback guidance
- remote PR disabled-by-default posture
- allowlisted test runner

Future management profile items:

- AI incident vocabulary
- containment and rollback playbooks
- emergency safe-mode procedure
- provider disablement workflow
- trust-root rotation procedure
- hard-deny readiness checklist

## Future config shape

Step 42 does not implement this config. It reserves a possible future shape:

```yaml
ai_risk:
  enabled: true
  profile: local_developer
  framework:
    nist_ai_rmf: true
    nist_genai_profile: true
  governance:
    owner: ""
    approvers: []
    risk_tolerance: low
  data_exposure:
    default_label: internal
    provider_allowed_labels:
      - public
      - internal
    secret_labels:
      - secret
      - credential
      - token
  inventory:
    require_provider_inventory: true
    require_pack_inventory: true
    require_capability_inventory: true
  measurement:
    require_policy_outcome_metrics: true
    require_redaction_metrics: true
    require_audit_health: true
  incident_response:
    require_incident_events: true
    require_rollback_guidance: true
```

## Future CLI shape

Step 42 does not implement these commands. They are reserved as possible future design targets:

```text
gadgets risk profile [--project <path>]
gadgets risk inventory [--project <path>]
gadgets risk report [--project <path>]
gadgets risk incidents [--project <path>]
gadgets risk rmf-map [--project <path>]
```

## Proposed future evidence artifacts

Future AI risk or governance reports may produce evidence artifacts such as:

```text
ai_rmf_alignment_summary.md
ai_system_inventory.yaml
provider_model_inventory.yaml
capability_zone_inventory.yaml
data_exposure_inventory.yaml
policy_outcome_metrics.yaml
redaction_metrics.yaml
trust_gate_metrics.yaml
audit_health_summary.yaml
incident_event_summary.yaml
rollback_guidance.md
```

## Proposed future audit event vocabulary

Future AI risk/governance events may include:

```text
ai.risk.profile.reviewed
ai.risk.inventory.reviewed
ai.risk.report.generated
ai.risk.incident.recorded
ai.risk.provider.disabled
ai.risk.provider.enabled
ai.risk.data_exposure.warning
ai.risk.data_exposure.denied
ai.risk.policy_metric.recorded
ai.risk.redaction_metric.recorded
ai.risk.audit_health.checked
```

## Non-goals for Step 42

Step 42 does not add:

- runtime code
- new CLI commands
- AI RMF compliance claim
- certification claim
- legal or regulatory advice
- provider-side authority
- provider-side tool execution
- broader Git behavior
- hard-deny pack-load enforcement
- signing tools
- trust-root mutation
- pack install/update behavior
- registry downloads
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior

## Recommended next step

Pause for external Rust validation if the next checkpoint will be release-oriented. If validation remains deferred, the next bounded step should either add docs-only AI risk inventory contracts or continue improving operator reporting without adding hard-deny enforcement.
