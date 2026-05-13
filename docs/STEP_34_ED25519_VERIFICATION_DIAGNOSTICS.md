# Step 34 - Ed25519 Verification Diagnostics

Date: 2026-05-13
Status: complete at checkpoint level; external Rust validation deferred

## Purpose

Step 34 adds real Ed25519 verification to the existing diagnostic `gadgets pack trust signature` command.

This is intentionally diagnostic and non-enforcing. It makes signed-pack verification observable and evidence-backed before the framework uses pack signatures as an authoritative Team or Production pack-load gate.

## Command

```bash
gadgets pack trust signature [--project <path>] <pack>
```

## Implemented behavior

For project-local packs, the diagnostic now checks:

- `pack.signature.yaml` presence and required metadata fields
- signature metadata version `1`
- algorithm `ed25519`
- strict UTC timestamp shape for `created_at` and `expires_at`
- expired signature metadata
- raw-byte SHA-256 over `pack.yaml`
- raw-byte SHA-256 over `pack.contents.yaml`
- `pack.contents.yaml` file entries
- safe relative content paths
- sorted content paths
- duplicate content paths
- listed file existence
- regular-file requirement
- symlink rejection
- listed file SHA-256 hashes
- required `pack.yaml` entry
- rejection of `pack.signature.yaml` as a signed content entry
- matching trust-root publisher
- matching trust-root key id
- matching trust-root algorithm
- trust-root allowed pack id
- expired trust-root publisher metadata
- base64 public key shape
- base64 signature shape
- Ed25519 signature verification over the deterministic `gadgets-pack-signature-v1` payload

Built-in packs remain trusted as part of the runtime distribution and do not require project-local signature files in this diagnostic.

## Signature payload

The verified payload is exactly the Step 33 payload:

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

The payload is encoded as UTF-8 bytes, uses LF line endings, and ends with one final LF.

## Evidence

The command writes diagnostic evidence including:

```text
signature_metadata_check.txt
signature_verification_result.txt
signature_payload_v1.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_signature_summary.yaml
trust_root_summary.yaml
signature_metadata_findings.txt
policy_mode.txt
```

The evidence records whether cryptographic verification was performed and whether it succeeded. It does not include private keys or signing seeds.

## Audit

The command continues to append:

```text
pack.signature.checked
evidence.created
```

Step 34 does not emit authoritative pack-load allow or deny events. Those remain future Team/Production enforcement work.

## Boundary

Step 34 does not add:

- pack trust enforcement
- signing tools
- trust-root mutation
- pack install/update commands
- registry downloads
- Team/Production pack-load enforcement
- Gadget execution behavior changes
- arbitrary shell
- Linux admin behavior
- database behavior
- cloud behavior
- deployment behavior

## Acceptance checklist

- [x] Ed25519 dependency added to the CLI crate.
- [x] Base64 dependency added to the CLI crate.
- [x] Signature metadata values are used to build the Step 33 payload.
- [x] Trust-root public keys are decoded from base64.
- [x] Signatures are decoded from base64.
- [x] Content manifest file hashes are verified.
- [x] Expired signature metadata is rejected in the diagnostic result.
- [x] Expired trust-root metadata is rejected in the diagnostic result.
- [x] Successful verification reports `trusted_signed` diagnostically.
- [x] Failed verification reports deterministic findings.
- [x] Evidence and audit are emitted.
- [x] Pack loading is not enforced.

## Validation note

External Rust validation was not rerun after Step 34. The last full external Rust validation baseline remains commit `c5fbd78`. Steps 24, 25, 27, 28, 30, 31, 32, and 34 include Rust source changes after that baseline.
