# Step 4 - Append-only Audit Ledger

Date: 2026-05-12

## Purpose

Step 4 adds the first concrete audit ledger behavior to Gadgets Framework.

The ledger records structured audit events as JSONL and verifies a simple hash chain so local tampering can be detected.

## Scope completed

Implemented in `crates/gadgets-ledger`:

- audit schema constant
- default local ledger path helper
- audit event constructor
- append-only event writer
- JSONL event reader
- event summary rows
- SHA-256 event hash calculation
- hash-chain verifier
- unit tests for empty, valid, and tampered ledgers

Implemented in `crates/gadgets-cli`:

- `gadgets ledger show [project-root-or-ledger-path]`
- `gadgets ledger verify [project-root-or-ledger-path]`
- `gadgets init` now creates `.gadgets/ledger/events.jsonl`

## Ledger file

Default path:

```text
.gadgets/ledger/events.jsonl
```

Each line is one JSON audit event.

## Hash-chain behavior

Each event stores:

- `previous_event_hash`
- `event_hash`

The event hash is computed over the serialized event with `event_hash` cleared. This makes the event content and previous hash binding verifiable.

The first event has no previous hash.

## CLI behavior

Show events:

```bash
gadgets ledger show
```

Verify hash chain:

```bash
gadgets ledger verify
```

Both commands accept either a project root or direct ledger path:

```bash
gadgets ledger verify /path/to/project
gadgets ledger verify /path/to/project/.gadgets/ledger/events.jsonl
```

## Safety properties

This step does not add runtime actions, provider calls, shell execution, filesystem inspection, evidence persistence, patch application, or Linux administration behavior.

It only creates and verifies audit records.

## Current limitations

- The ledger is local JSONL, not a distributed immutable log.
- Verification detects tampering but does not prevent deletion.
- No SIEM export or retention policy exists yet.
- Runtime components do not yet emit real workflow events.

## Next step

Step 5 should implement the evidence bundle writer for observe-only runs.
