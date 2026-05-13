# Step 32 - Signature Metadata Verification Scaffold

Date: 2026-05-13

## Scope

Step 32 adds a non-enforcing signature metadata verification scaffold for pack trust diagnostics.

New command:

```bash
gadgets pack trust signature [--project <path>] <pack>
```

The command validates the shape and internal consistency of `pack.signature.yaml` metadata and checks whether the signature metadata references a matching local trust-root publisher/key record.

## What the command checks

For project-local packs, the diagnostic checks:

- `pack.signature.yaml` exists
- signature metadata version is `1`
- algorithm is `ed25519`
- publisher is present
- key id is present
- pack id matches the loaded pack name
- pack version matches the loaded pack version
- manifest SHA-256 matches the loaded `pack.yaml`
- contents SHA-256 matches `pack.contents.yaml` when present
- `created_at` uses strict UTC timestamp format
- `expires_at` uses strict UTC timestamp format
- signature value is present
- trust-root metadata contains matching publisher, key id, and algorithm
- trust-root metadata allows the pack id

Strict UTC timestamp format is:

```text
YYYY-MM-DDTHH:MM:SSZ
```

## Evidence

The command writes a normal diagnostic evidence bundle with these artifacts:

```text
summary.md
bundle.yaml
signature_metadata_check.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_signature_summary.yaml
trust_root_summary.yaml
signature_metadata_findings.txt
policy_mode.txt
```

## Audit

The command appends:

```text
pack.signature.checked
evidence.created
```

## Boundaries preserved

Step 32 does not add:

- cryptographic signature verification
- signing tools
- trust enforcement
- trust-root mutation
- pack install/update behavior
- registry downloads
- Team/Production enforcement
- Gadget execution behavior changes
- arbitrary shell
- Linux admin behavior
- database/cloud/deployment behavior

The command is diagnostic only. It is intended to prepare the data model, evidence, audit, and CLI behavior needed before real cryptographic verification is introduced later.
