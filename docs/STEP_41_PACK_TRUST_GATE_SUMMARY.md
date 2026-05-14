# Step 41 - Pack Trust Gate Summary Reporting

Date: 2026-05-13

## Goal

Add a read-only operator summary for pack-load trust gate outcomes.

Step 37 records runtime warning and dry-run denial events. Step 38 can preview a specific pack and operation. Step 39 lists matching trust-gate events from the audit ledger. Step 40 reports the configured gate posture. Step 41 adds a compact summary that helps an operator understand whether there is enough clean dry-run history to discuss hard-deny later.

## Command

```bash
gadgets pack trust gate-summary [--project <path>]
```

## Behavior

The command reads local project configuration and the local audit ledger, then reports:

- project path
- active runtime mode
- pack trust enabled state
- configured enforcement for the active mode
- effective Step 37 enforcement for the active mode
- hard-deny deferral status
- evidence and audit requirements
- ledger path
- trust-gate event counts
- review posture

The trust-gate event counts include:

- `pack.trust.gate.previewed`
- `pack.trust.warning`
- `pack.trust.dry_run_denied`
- `pack.trust.denied`
- `pack.load.denied`

The command intentionally ignores `evidence.created` so the summary focuses on trust decisions rather than supporting evidence lifecycle records.

## Review posture values

The command prints one posture string:

| Posture | Meaning |
|---|---|
| `not_ready_gate_disabled_or_off` | The active mode does not have an effective Step 37 gate. Do not discuss hard-deny from this state. |
| `not_ready_dry_run_denials_present` | The ledger has dry-run or denial findings. Review and remediate before hard-deny. |
| `review_warnings_before_hard_deny` | Safe Mode warnings exist. Review local unsigned or mixed-source pack usage before hard-deny planning. |
| `candidate_for_hard_deny_review_after_validation` | Preview activity exists and no warning or denial events were found in the local ledger. This is only a review signal; external Rust validation and operator approval are still required. |
| `collect_dry_run_evidence_before_hard_deny` | No trust-gate events exist yet. Run preview and dry-run workflows before hard-deny is considered. |

## Boundaries

Step 41 does not:

- hard-deny pack loading
- execute Gadgets
- write evidence
- append audit events
- verify signatures
- mutate trust roots
- install packs
- update packs
- download registry content
- add signing tools
- add Linux admin behavior
- add database behavior
- add cloud behavior
- add deployment behavior
- broaden Git behavior
- allow provider-side action bypass

## Validation status

External Rust validation remains deferred by user request. This checkpoint should be validated together with Steps 37 through 41 when validation resumes.
