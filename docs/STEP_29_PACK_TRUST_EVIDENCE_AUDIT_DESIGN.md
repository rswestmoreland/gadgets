# Step 29 - Pack Trust Evidence and Audit Design

Date: 2026-05-13

## Status

Design checkpoint complete. No runtime enforcement code was added.

Step 29 defines the audit events and evidence artifacts that future pack trust verification and enforcement must emit. This keeps pack trust observable and reviewable before it becomes a hard runtime gate.

## Scope

This step documents:

- pack trust audit event names
- trust-root audit event names
- signature verification audit event names
- evidence artifact names for trust checks
- evidence artifact names for trust-root inspection
- failure-recording behavior for future enforcement
- redaction and key-material handling rules
- rollout order for future implementation

## Non-goals

Step 29 does not implement:

- cryptographic signature verification
- pack trust enforcement
- pack signing tools
- trust-root editing commands
- pack install or update commands
- registry downloads
- Team or Production mode enforcement
- Gadget execution changes
- arbitrary shell
- Linux admin, database, cloud, or deployment behavior

## Audit event contract

Future pack trust behavior should use stable event names so pack decisions can be reviewed across diagnostics, enforcement, and later Team workflows.

Recommended pack trust events:

```text
pack.trust.checked
pack.trust.allowed
pack.trust.denied
pack.trust.warning
```

Recommended signature events:

```text
pack.signature.checked
pack.signature.verified
pack.signature.failed
pack.signature.expired
```

Recommended trust-root events:

```text
trust.root.loaded
trust.root.missing
trust.root.rejected
trust.root.expired
trust.root.warning
```

Recommended enforcement event:

```text
pack.load.denied
```

Diagnostic-only commands may emit checked/warning style events later if they gain run contexts. Enforcement paths must emit either allowed or denied when pack trust becomes authoritative for Team or Production mode.

## Evidence artifact contract

Future pack trust checks should write small, reviewable artifacts. They must not copy private keys, API tokens, or full secret-bearing configs into evidence.

Recommended pack trust evidence artifacts:

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

Recommended trust-root evidence artifacts:

```text
summary.md
bundle.yaml
trust_root_path.txt
trust_root_summary.yaml
trusted_publishers_summary.txt
trust_root_findings.txt
```

Recommended future enforcement-denial artifacts:

```text
summary.md
bundle.yaml
pack_load_denial.txt
pack_trust_decision.txt
trust_findings.txt
requested_gadget.txt
requested_capability.txt
runtime_mode.txt
```

## Redaction and key-material rules

Evidence and audit must never include:

- private keys
- signing seeds
- API tokens
- full secret-bearing config files
- raw provider credentials

Public keys may appear in diagnostics only if future policy allows it. The safer default is to record key IDs, algorithms, publisher names, expiration times, and hashes instead of full key material.

## Failure behavior for future enforcement

When pack trust enforcement is implemented, a denied pack must fail before Gadget execution.

A trust denial should:

- prevent the Gadget from being loaded for execution
- prevent provider-requested handoffs to the denied pack from executing
- record the trust decision in audit
- write trust-denial evidence when a run context exists
- return a clear human-readable error
- avoid falling back silently to older pack contents

## Rollout recommendation

Recommended next implementation order after Step 29:

1. Add shared trust decision and evidence structs.
2. Add evidence output for diagnostic trust checks.
3. Add audit output for diagnostic trust checks when a run context exists.
4. Add signed-pack parsing with deterministic canonicalization locked.
5. Add cryptographic signature verification.
6. Enforce trust only for Team or Production mode first.
7. Add Safe Mode warnings for unsigned local packs.
8. Add signing/trust-root mutation tooling later.

## Acceptance for this checkpoint

- Pack trust evidence artifact names are documented.
- Pack trust audit event names are documented.
- Trust-root evidence artifact names are documented.
- Signature verification audit event names are documented.
- Future enforcement denial behavior is documented.
- No enforcement code is added.
- No signing tools are added.
- No trust-root mutation is added.
- No runtime trust behavior changes are made.

## Step 30 follow-up

Step 30 implements diagnostic evidence and audit emission for `gadgets pack trust check` and `gadgets pack trust roots`. The Step 29 design remains the contract for future signature verification and enforcement behavior.
