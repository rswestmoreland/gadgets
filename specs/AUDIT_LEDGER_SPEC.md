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

## Step 36 future pack-load trust gate audit events

Step 36 defines the audit vocabulary for the future dry-run and hard-deny pack-load gate.

Future gate events:

```text
pack.trust.checked
pack.trust.allowed
pack.trust.warning
pack.trust.dry_run_denied
pack.trust.denied
pack.load.denied
pack.signature.checked
pack.signature.verified
pack.signature.failed
pack.signature.expired
trust.root.loaded
trust.root.missing
trust.root.rejected
trust.root.expired
evidence.created
```

Audit records should include safe identifiers only: pack id, pack version, source class, manifest hash, contents hash, publisher, key id, runtime mode, enforcement state, decision kind, run id when present, loaded manifest source summary, and findings.

Audit records must not include private keys, signing seeds, API tokens, provider credentials, secret-bearing config values, or full unredacted provider output.

When the future gate is active, failure to append required audit for project-local or mixed-source runtime actions must fail closed.

## Step 37 pack-load trust dry-run audit

Step 37 emits runtime dry-run gate audit records for project-local and mixed-source pack material when the gate records a warning or would-deny result.

Implemented dry-run events:

```text
pack.trust.warning
pack.trust.dry_run_denied
evidence.created
```

Step 37 does not emit `pack.trust.denied` or `pack.load.denied`, because hard-deny enforcement remains deferred.

If the gate is active and required audit cannot be appended for project-local or mixed-source runtime actions, the runtime fails closed before executing the Gadget action.


## Step 38 pack-load trust gate preview audit

Step 38 adds a diagnostic audit event for operator previews of the runtime pack-load trust gate:

```text
pack.trust.gate.previewed
```

This event records that the configured gate outcome was previewed for an installed pack and optional operation. It is not a runtime warning event, dry-run denial event, hard-denial event, or pack-load denial event.

Step 38 also appends the existing `evidence.created` event for the diagnostic evidence bundle.


## Step 39 pack-load trust gate history reporting

Step 39 adds a read-only history view over existing audit records:

```text
gadgets pack trust gate-history [--project <path>] [--limit <n>]
```

The command filters these event types:

```text
pack.trust.warning
pack.trust.dry_run_denied
pack.trust.gate.previewed
pack.trust.denied
pack.load.denied
```

It intentionally excludes `evidence.created` so the view focuses on trust-gate decisions rather than evidence bundle side effects. The command does not append new audit events and does not write evidence.


## Step 40 pack trust gate status audit behavior

Step 40 adds a read-only status command:

```text
gadgets pack trust gate-status [--project <path>]
```

This command must not append audit events. It reads configuration only and reports the current trust-gate posture.

## Step 41 pack trust gate summary audit behavior

Step 41 adds a read-only summary command:

```bash
gadgets pack trust gate-summary [--project <path>]
```

The command reads prior audit events and summarizes pack-load trust gate event counts. It must not append new audit events. It must ignore `evidence.created` when counting trust decisions so evidence lifecycle events do not inflate trust-gate outcome counts.
