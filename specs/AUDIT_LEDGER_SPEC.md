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
