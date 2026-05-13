# Step 30 - Pack Trust Diagnostic Evidence Emission

Date: 2026-05-13

## Status

Implementation checkpoint complete.

Step 30 adds evidence and audit output to the existing non-enforcing pack trust diagnostics:

```bash
gadgets pack trust check [--project <path>] <pack>
gadgets pack trust roots [--project <path>]
```

Both commands remain diagnostic only.

## What changed

`gadgets pack trust check` now:

- generates a run id
- writes a normal evidence bundle under `.gadgets/runs/<run-id>/evidence`
- appends `pack.trust.checked`
- appends `evidence.created`
- prints the run id, evidence path, and ledger path

`gadgets pack trust roots` now:

- generates a run id
- writes a normal evidence bundle under `.gadgets/runs/<run-id>/evidence`
- appends `trust.root.loaded` when the trust-root file exists
- appends `trust.root.missing` when the trust-root file is absent
- appends `evidence.created`
- prints the run id, evidence path, and ledger path

## Pack trust check evidence

The diagnostic evidence bundle includes:

```text
summary.md
bundle.yaml
pack_trust_decision.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_contents_summary.txt
pack_signature_summary.yaml
trust_root_summary.txt
trust_findings.txt
policy_mode.txt
```

The evidence records trust metadata and findings only. It does not copy private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs.

## Trust-root inspection evidence

The diagnostic evidence bundle includes:

```text
summary.md
bundle.yaml
trust_root_path.txt
trust_root_summary.yaml
trusted_publishers_summary.txt
trust_root_findings.txt
```

The publisher summary records publisher name, key id, algorithm, public-key presence, allowed-pack count, and expiration timestamp. It does not copy public key material into evidence.

## Audit events

Pack trust check emits:

```text
pack.trust.checked
evidence.created
```

Trust-root inspection emits one of:

```text
trust.root.loaded
trust.root.missing
```

and also emits:

```text
evidence.created
```

These events are diagnostic events. They are not enforcement allow/deny decisions.

## Preserved boundaries

Step 30 does not implement:

- cryptographic signature verification
- pack trust enforcement
- signing tools
- trust-root editing
- pack install or update commands
- registry downloads
- Team or Production mode enforcement
- Gadget execution behavior changes
- arbitrary shell
- Linux admin, database, cloud, or deployment behavior

## Why this step matters

Pack trust will eventually become a runtime gate for Team and Production modes. Before enforcement is added, trust diagnostics need durable evidence and audit records so trust decisions are reviewable and can be tested safely.

## Acceptance for this checkpoint

- `gadgets pack trust check` emits diagnostic evidence.
- `gadgets pack trust roots` emits diagnostic evidence.
- Pack trust diagnostics append audit events.
- Trust-root diagnostics append audit events.
- Evidence avoids key material and secret-bearing configs.
- Commands remain non-enforcing and non-mutating.
- No signing, verification, install, download, or enforcement behavior is added.
