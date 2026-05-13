# Step 26 - Pack Trust and Signing Design

Status: complete as a docs-first design checkpoint. Enforcement code is not implemented yet.

## Goal

Define how packs become trusted before the project supports broader third-party packs, Team Mode pack enforcement, or Production Mode pack enforcement.

The main contract is in `specs/PACK_TRUST_SIGNING_SPEC.md`.

## Design summary

Pack trust answers whether a pack is eligible to be loaded and used. It does not give a pack authority to bypass runtime policy.

A trusted pack still must pass:

- Gadget manifest validation
- capability checks
- tool allowlist checks
- zone and path boundary checks
- runtime mode restrictions
- approval requirements
- evidence creation
- audit logging

Provider output remains untrusted even if it targets a trusted pack.

## Locked design decisions

### Pack identity

Pack identity is based on:

- pack id
- version
- publisher
- source type
- source URI
- canonical `pack.yaml` hash
- content manifest hash

The content hash is part of the effective identity. A publisher cannot safely reuse the same version with different contents without being treated as a different artifact.

### Content manifest

Signed packs should include a deterministic `pack.contents.yaml` file listing each trusted file and SHA-256 hash.

The content manifest must use normalized paths and must reject parent traversal.

### Signature record

Signed packs should include a detached `pack.signature.yaml` record.

Recommended algorithm choices:

- SHA-256 for content hashes
- Ed25519 for signatures
- strict UTC RFC3339 timestamps without fractional seconds

The exact canonical signature payload remains a future implementation detail.

### Trust roots

Project trust roots should live under:

```text
.gadgets/trust/trusted_publishers.yaml
```

Team or Production deployments can later introduce centrally managed trust roots.

### Runtime mode behavior

Safe mode keeps local development workable:

- built-in packs are allowed
- project-local unsigned packs may be allowed only with explicit local config and audit warning
- signed project-local packs are allowed if signature verification succeeds

Team mode should require signed non-built-in packs unless an explicit team exception exists.

Production mode should fail closed:

- unsigned non-built-in packs denied
- unknown publishers denied
- expired trust roots denied
- expired signatures denied
- signature/content mismatch denied

## Verification outcomes

The design defines these future outcomes:

- `trusted_builtin`
- `trusted_signed`
- `allowed_unsigned_local`
- `denied_unsigned`
- `denied_unknown_publisher`
- `denied_unknown_key`
- `denied_signature_mismatch`
- `denied_content_mismatch`
- `denied_expired_signature`
- `denied_expired_trust_root`
- `denied_invalid_manifest`

## Future audit events

Recommended events:

- `pack.trust.checked`
- `pack.trust.allowed`
- `pack.trust.denied`
- `pack.signature.verified`
- `pack.signature.failed`
- `trust.root.loaded`
- `trust.root.rejected`

## Step 26 non-goals

Step 26 does not add:

- signature verification code
- signing tools
- registry downloads
- pack install/update commands
- trust-root mutation commands
- Team/Production enforcement
- third-party pack execution

## Recommended next implementation step

Proceed with a non-mutating trust-inspection scaffold before enforcement:

```text
gadgets pack trust check [--project <path>] <pack>
```

The first implementation should report built-in vs project-local vs unsigned status, but it should not yet enforce signatures or require external cryptography.
