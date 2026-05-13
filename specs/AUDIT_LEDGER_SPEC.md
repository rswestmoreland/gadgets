# Audit Ledger Specification

Step 7 update: observe-only filesystem runs append `run.started`, `handoff.allowed`, `action.allowed`, `action.denied`, `evidence.created`, and `run.completed` events.



Implementation status: Step 4 adds local JSONL ledger append/read/verify behavior in `crates/gadgets-ledger` and CLI commands in `crates/gadgets-cli`.


The audit ledger records what happened across the Gadgets runtime.

Initial local path:

```text
.gadgets/ledger/events.jsonl
```

## Event requirements

Each event should include:

- event_id
- schema_version
- timestamp
- event_type
- actor
- run_id
- decision
- summary
- previous_event_hash
- event_hash

The ledger is append-only. Corrections are new events referencing old events.

## Pack trust audit events

Step 29 defines the audit vocabulary for pack trust and signing. Step 30 emits diagnostic-only audit events for `gadgets pack trust check` and `gadgets pack trust roots`. Step 31 emits diagnostic-only `pack.trust.policy.previewed` events for `gadgets pack trust preview`. Step 32 emits diagnostic-only `pack.signature.checked` events for `gadgets pack trust signature`. Step 35 keeps the same preview event but records signature-aware preview findings in evidence. These events are not enforcement decisions yet; they record diagnostics and evidence creation only.

Pack trust events:

- `pack.trust.checked`
- `pack.trust.allowed`
- `pack.trust.denied`
- `pack.trust.warning`

Signature events:

- `pack.signature.checked`
- `pack.signature.verified`
- `pack.signature.failed`
- `pack.signature.expired`

Trust-root events:

- `trust.root.loaded`
- `trust.root.missing`
- `trust.root.rejected`
- `trust.root.expired`
- `trust.root.warning`

Future enforcement event:

- `pack.load.denied`

Pack trust audit records must not include private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs. Diagnostic commands currently emit `pack.trust.checked`, `trust.root.loaded` or `trust.root.missing`, and `evidence.created`. Future enforcement may emit allowed/denied/failure events once signature verification is implemented.

## Step 32 signature metadata diagnostic events

`gadgets pack trust signature [--project <path>] <pack>` emits diagnostic-only audit events:

- `pack.signature.checked`
- `evidence.created`

These events record signature metadata shape and trust-root reference diagnostics only. They are not cryptographic verification events and must not be treated as enforcement evidence.

## Step 33 future cryptographic verification audit events

Step 33 finalizes the expected audit behavior for future real signature verification. A real verifier should emit outcome-specific events such as:

- `pack.signature.checked`
- `pack.signature.verified`
- `pack.signature.failed`
- `pack.signature.expired`
- `trust.root.loaded`
- `trust.root.expired`
- `pack.trust.allowed`
- `pack.trust.denied`
- `evidence.created`

Audit records must not include private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs. They should prefer publisher names, key ids, algorithms, hashes, validity timestamps, decision kinds, and findings.


## Step 34 signature verification diagnostic audit

`gadgets pack trust signature` continues to emit `pack.signature.checked` and `evidence.created` for the diagnostic path. Step 34 records Ed25519 verification findings in the evidence bundle but does not emit pack-load allow/deny enforcement events. Future enforcement may add `pack.signature.verified`, `pack.signature.failed`, `pack.trust.allowed`, and `pack.trust.denied` when those events become authoritative gates.

## Step 35 signature-aware policy preview audit

`gadgets pack trust preview` continues to emit `pack.trust.policy.previewed` and `evidence.created`. Step 35 does not add authoritative allow/deny events. Signature-aware preview outcomes are recorded in evidence, not as enforcement decisions.
