# Step 33 - Cryptographic Verification Design Finalization

Date: 2026-05-13
Status: complete as a docs-first design checkpoint

## Purpose

Step 33 locks the implementation design for real pack signature verification before adding cryptographic code or enforcement.

The goal is to remove ambiguity from the next implementation step. After Step 33, future code should know exactly what is being verified, which bytes are signed, how hashes are recomputed, which trust-root fields are required, and how failures map to decisions, evidence, and audit events.

## Boundary

Step 33 is design only.

It does not add:

- cryptographic signature verification code
- signing tools
- pack trust enforcement
- trust-root mutation
- pack install or update behavior
- registry downloads
- Team or Production mode runtime enforcement
- Gadget execution behavior changes
- arbitrary shell
- Linux admin, database, cloud, or deployment behavior

## Locked cryptographic choices

Step 33 locks the first implementation target to:

- signature algorithm: Ed25519
- hash algorithm: SHA-256
- digest encoding: lowercase hex
- public key encoding: base64 without line breaks
- signature encoding: base64 without line breaks
- timestamp format: strict UTC RFC3339 without fractional seconds
- timestamp example: `2026-05-13T00:00:00Z`

Future algorithms may be added later by versioned policy, but version 1 verification should support only Ed25519 and SHA-256.

## Files involved

For a project-local signed pack, verification uses these files:

```text
pack.yaml
pack.contents.yaml
pack.signature.yaml
.gadgets/trust/trusted_publishers.yaml
```

Built-in runtime packs remain trusted as part of the runtime distribution. They do not require project-local signature files in the first implementation.

## Raw-byte hash contract

Step 33 intentionally avoids YAML canonicalization for the cryptographic hash inputs.

The verifier recomputes:

```text
manifest_sha256 = sha256(raw bytes of pack.yaml)
contents_sha256 = sha256(raw bytes of pack.contents.yaml)
```

The verifier must not normalize YAML, reorder keys, trim whitespace, or rewrite line endings before hashing these files.

This makes the signed artifact identity easy to reproduce: the exact bytes on disk are the bytes being hashed.

## Content manifest verification

`pack.contents.yaml` lists the pack files and their SHA-256 hashes.

Before signature verification succeeds, the verifier must also validate the content manifest:

1. `pack.contents.yaml` exists.
2. It parses as version `1`.
3. `pack_id` matches `pack.yaml` and `pack.signature.yaml`.
4. File paths are relative to the pack root.
5. File paths do not contain parent traversal.
6. File paths are sorted in deterministic order.
7. Duplicate file paths are rejected.
8. Every listed file exists.
9. Every listed file hash matches the raw bytes of that file.
10. `pack.yaml` is listed with its matching hash.
11. `pack.signature.yaml` is not included in the signed contents list.
12. Runtime-generated files are not included.
13. Symlinks are rejected unless future policy explicitly supports them.

The detached signature signs the `contents_sha256`, so the file list and file hashes are protected through the hash of `pack.contents.yaml`.

## Signature payload v1

The Ed25519 signature is over a deterministic ASCII payload, encoded as UTF-8 bytes.

The version 1 payload is line based, uses LF line endings, has no trailing spaces, and ends with exactly one final LF.

Payload format:

```text
gadgets-pack-signature-v1
algorithm:ed25519
publisher:<publisher>
key_id:<key_id>
pack_id:<pack_id>
pack_version:<pack_version>
manifest_sha256:<manifest_sha256>
contents_sha256:<contents_sha256>
created_at:<created_at>
expires_at:<expires_at>
```

Rules:

- Field order is fixed exactly as shown.
- Field names are lowercase exactly as shown.
- Values are copied from validated metadata after basic shape validation.
- No YAML serialization is used for the payload.
- No extra fields are included in v1.
- Missing or malformed fields deny verification.
- Unknown fields in `pack.signature.yaml` may be ignored for v1, but they are not part of the signed payload.

This payload signs the expected publisher/key identity, pack identity, artifact hashes, and validity window.

## Verification flow

The first real verification implementation should follow this order:

1. Locate the pack source.
2. If the pack is built-in, return `trusted_builtin` and record that no project signature was required.
3. For project-local packs, read `pack.yaml`.
4. Read `pack.contents.yaml`.
5. Read `pack.signature.yaml`.
6. Read `.gadgets/trust/trusted_publishers.yaml`.
7. Validate signature metadata shape.
8. Validate strict UTC timestamp shape for `created_at` and `expires_at`.
9. Reject signatures that are not yet valid if future `not_before` is added; v1 has no `not_before`.
10. Reject expired signatures.
11. Recompute raw-byte `manifest_sha256`.
12. Recompute raw-byte `contents_sha256`.
13. Validate every content manifest file hash.
14. Match publisher, key id, algorithm, and allowed pack id against the trust root.
15. Reject expired trust roots.
16. Construct the exact payload v1 bytes.
17. Verify the Ed25519 signature over the payload bytes using the matched trust-root public key.
18. Return `trusted_signed` only if all checks pass.

Every failure must produce a deterministic denial kind and human-readable finding.

## Denial mapping

Recommended denial mapping:

| Condition | Decision |
|---|---|
| Missing `pack.signature.yaml` | `denied_unsigned` in Team/Production; warning in Safe Mode |
| Missing `pack.contents.yaml` | `denied_invalid_manifest` |
| Invalid content path | `denied_invalid_manifest` |
| Listed file hash mismatch | `denied_content_mismatch` |
| `pack.yaml` hash mismatch | `denied_signature_mismatch` |
| `pack.contents.yaml` hash mismatch | `denied_content_mismatch` |
| Unknown publisher | `denied_unknown_publisher` |
| Unknown key id | `denied_unknown_key` |
| Unsupported algorithm | `denied_unknown_key` or `denied_invalid_manifest` |
| Expired signature | `denied_expired_signature` |
| Expired trust root | `denied_expired_trust_root` |
| Ed25519 verification failure | `denied_signature_mismatch` |

Safe Mode may still allow unsigned project-local packs with warnings according to the existing Safe Mode design. Team and Production previews and future enforcement should fail closed for unsigned or invalid signed packs.

## Evidence design for real verification

Future cryptographic verification should write evidence that proves what was checked without exposing private material.

Recommended artifacts:

```text
signature_verification_result.txt
signature_payload_v1.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_contents_hash.txt
pack_contents_verification.txt
pack_signature_summary.yaml
trust_root_match.txt
signature_verification_findings.txt
policy_mode.txt
```

Evidence must not include:

- private keys
- signing seeds
- API tokens
- provider credentials
- full secret-bearing configs

Including the public key itself is not required. Prefer publisher, key id, algorithm, trust-root expiration, and hashes.

## Audit events for real verification

Future cryptographic verification should emit:

```text
pack.signature.checked
pack.signature.verified
pack.signature.failed
pack.signature.expired
trust.root.loaded
trust.root.expired
pack.trust.allowed
pack.trust.denied
evidence.created
```

The exact event set depends on outcome. Failure paths should be auditable even when pack loading is denied.

## Enforcement rollout

Recommended next implementation stages:

1. Add reusable verification result structs and canonical payload builder.
2. Add Ed25519 verification for `gadgets pack trust signature` diagnostics only.
3. Add evidence artifacts for the real verification result.
4. Update `gadgets pack trust preview` to use real verification results.
5. Add Safe Mode warning behavior using real verification.
6. Add Team/Production pack-load enforcement later.
7. Add signing tools only after verification has been validated.

## Acceptance criteria for Step 33

- The signature payload format is locked.
- The raw-byte hash contract is locked.
- Content manifest verification rules are locked.
- Trust-root matching rules are locked.
- Denial mappings are locked.
- Evidence and audit expectations for real verification are locked.
- No cryptographic code or enforcement is added in Step 33.
