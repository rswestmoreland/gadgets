# Pack Trust and Signing Spec

Status: Step 26 design contract plus Step 27/28 non-enforcing inspection scaffolds, Step 29 evidence/audit design, Step 30 diagnostic evidence/audit emission, Step 31 non-enforcing policy preview, Step 32 signature metadata diagnostics, Step 33 cryptographic verification design finalization, Step 34 non-enforcing Ed25519 verification diagnostics, and Step 35 signature-aware policy preview diagnostics. Pack trust enforcement and signing tools are not implemented yet.

## Purpose

Pack trust controls whether a pack is eligible to be loaded and used by the Gadgets runtime.

A trusted pack still does not execute by itself. Every action remains subject to deterministic policy, capability checks, tool allowlists, zone boundaries, evidence, audit, approval state, and runtime mode.

Signing answers a supply-chain question: "Did this pack come from an expected publisher and did its contents change?" It does not grant unlimited authority.

## Trust principles

- Built-in runtime packs are trusted by the runtime distribution.
- Project-local unsigned packs are allowed only in developer-oriented local use.
- Team and Production modes should require explicit trust records for non-built-in packs.
- Trust is evaluated before Gadget execution.
- Trust failures must deny execution, not degrade silently.
- Trust decisions must be auditable.
- A signature cannot expand a Gadget beyond its manifest capabilities, tools, zones, and permission level.
- A pack update must be treated as a new artifact identity.
- Pack trust must be deterministic and offline-verifiable once trust roots are configured.

## Pack identity

A pack identity is the stable tuple used in trust decisions.

Recommended fields:

```yaml
pack_identity:
  id: developer
  version: 0.1.0
  publisher: gadgets-framework
  source_type: builtin
  source_uri: builtin://developer
  manifest_sha256: <sha256-of-pack-yaml>
  contents_sha256: <sha256-of-pack-content-manifest>
```

Field meanings:

- `id`: canonical pack id, such as `developer` or `linux-admin-observe`.
- `version`: semantic version or local development version.
- `publisher`: publisher namespace used for trust roots.
- `source_type`: one of `builtin`, `project_local`, `registry`, or `archive`.
- `source_uri`: human-readable origin reference.
- `manifest_sha256`: hash of canonicalized `pack.yaml`.
- `contents_sha256`: hash of the full content manifest for the pack.

The effective pack identity used by policy should include the content hash. A publisher reusing the same version with different contents is treated as a different artifact and should require explicit handling.

## Content manifest

A signed pack should include a deterministic content manifest listing every trusted file in the pack.

Recommended file name:

```text
pack.contents.yaml
```

Recommended shape:

```yaml
version: 1
pack_id: developer
files:
  - path: pack.yaml
    sha256: <hex>
  - path: gadgets/filesystem.read.yaml
    sha256: <hex>
  - path: gadgets/patch.writer.yaml
    sha256: <hex>
```

Rules:

- Paths are repo-relative to the pack root.
- Paths must be normalized and must not contain parent traversal.
- Hidden files are excluded unless explicitly listed.
- Symlinks are not trusted unless future policy explicitly supports them.
- The manifest is sorted by path before hashing/signing.
- Runtime-generated files are excluded.

## Signed pack record

A signed pack should include a detached signature record.

Recommended file name:

```text
pack.signature.yaml
```

Recommended shape:

```yaml
version: 1
algorithm: ed25519
publisher: gadgets-framework
key_id: <publisher-key-id>
pack_id: developer
pack_version: 0.1.0
manifest_sha256: <hex>
contents_sha256: <hex>
created_at: 2026-05-13T00:00:00Z
expires_at: 2027-05-13T00:00:00Z
signature: <base64-signature>
```

Recommended cryptographic choices:

- Hash: SHA-256.
- Signature: Ed25519.
- Timestamps: strict UTC RFC3339 without fractional seconds.
- Signature payload: canonical pack identity plus content-manifest hashes.

Step 33 locks the version 1 signature payload format. This spec now defines the wire-compatible payload for the first cryptographic verification implementation.

## Trust roots

Trust roots define which publisher keys are accepted.

Recommended project trust file:

```text
.gadgets/trust/trusted_publishers.yaml
```

Recommended shape:

```yaml
version: 1
trusted_publishers:
  - publisher: gadgets-framework
    key_id: <publisher-key-id>
    algorithm: ed25519
    public_key: <base64-public-key>
    allowed_pack_ids:
      - developer
      - linux-admin-observe
    expires_at: 2027-05-13T00:00:00Z
```

Trust-root rules:

- Built-in runtime packs may use an embedded runtime trust root.
- Project-local trust roots apply only to that project.
- Team/Production deployments should support centrally managed trust roots later.
- Trust roots must be read-only inputs during action execution.
- Trust-root changes should require audit events in future implementation.

## Mode behavior

### Safe mode

Safe mode is the current local default.

Recommended behavior:

- Built-in packs: allowed.
- Project-local unsigned packs: allowed only with explicit local config and audit warning.
- Signed project-local packs: allowed if the signature verifies against configured trust roots.
- Signature failure: deny signed-pack loading.

This keeps local development convenient while still making trust status visible.

### Team mode

Recommended behavior:

- Built-in packs: allowed.
- Unsigned project-local packs: denied unless an explicit team policy exception exists.
- Signed packs: required for non-built-in packs.
- Unknown publisher/key: deny.
- Expired trust root or signature: deny.

### Production mode

Recommended behavior:

- Built-in packs: allowed only if runtime distribution is trusted.
- Non-built-in packs: must be signed by a configured trust root.
- Unsigned packs: deny.
- Developer exceptions: deny.
- Signature failure: deny.
- Expired key/signature: deny.

Production mode must prefer fail-closed behavior.

## Verification outcomes

Recommended trust decision kinds:

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

Trust verification should produce findings suitable for evidence and audit.

## Audit events

Step 29 locks the recommended audit event vocabulary for pack trust work. Step 30 emits diagnostic-only audit events for trust checks and trust-root inspection.

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

Enforcement event:

- `pack.load.denied`

Pack trust audit events should never include private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs. They should record stable identifiers, hashes, key IDs, publisher names, decision kinds, and human-readable findings.

## Evidence artifacts

Step 29 locks the recommended evidence artifact vocabulary for pack trust work. Step 30 emits the diagnostic trust-check and trust-root evidence artifacts.

Pack trust check evidence:

- `summary.md`
- `bundle.yaml`
- `pack_trust_decision.txt`
- `pack_identity.yaml`
- `pack_manifest_hash.txt`
- `pack_contents_summary.txt`
- `pack_signature_summary.yaml`
- `trust_root_summary.txt`
- `trust_findings.txt`
- `policy_mode.txt`

Trust-root inspection evidence:

- `summary.md`
- `bundle.yaml`
- `trust_root_path.txt`
- `trust_root_summary.yaml`
- `trusted_publishers_summary.txt`
- `trust_root_findings.txt`

Future pack-load denial evidence:

- `summary.md`
- `bundle.yaml`
- `pack_load_denial.txt`
- `pack_trust_decision.txt`
- `trust_findings.txt`
- `requested_gadget.txt`
- `requested_capability.txt`
- `runtime_mode.txt`

Do not copy private keys or complete secret-bearing configs into evidence. Prefer key IDs, publisher names, algorithms, expiration timestamps, and hashes over complete key material.

## Failure behavior

Trust failure must happen before Gadget execution.

If a pack is denied:

- do not load the denied Gadget for execution
- do not fall back to an older pack version silently
- do not execute provider-requested actions targeting that pack
- write audit evidence for the denial when a run context exists
- provide a clear human-readable error

## Rollout plan

Recommended implementation order:

1. Add trust metadata structs and verification result types.
2. Add non-enforcing trust inspection command. Completed in Step 27.
3. Add `gadgets pack trust check <pack>` for local diagnostics. Completed in Step 27.
4. Add `gadgets pack trust roots [--project <path>]` for non-mutating trust-root diagnostics. Completed in Step 28.
5. Add signed-pack file parsing without enforcement. Partially scaffolded in Step 27 and Step 32 for metadata inspection only.
6. Add diagnostic evidence emission for pack trust and trust-root inspection. Completed in Step 30.
7. Add non-enforcing policy preview. Completed in Step 31.
8. Lock the cryptographic verification byte contract. Completed in Step 33.
9. Add Ed25519 verification diagnostics without enforcement. Completed in Step 34.
10. Enforce trust only for Team/Production mode after diagnostics are validated.
11. Add Safe mode unsigned local warnings.
12. Add signing tooling later, after verification behavior is stable.

## Non-goals for Step 26 and Step 27

Step 26 is design-only. Step 27 is diagnostic inspection only. These steps do not implement:

- signature generation
- pack-load signature enforcement
- registry downloads
- package installation
- pack updates
- trust-root editing commands
- third-party pack execution enforcement
- Team/Production trust enforcement


### Step 28 trust root inspection behavior

`gadgets pack trust roots [--project <path>]` is diagnostic only. It reports whether `.gadgets/trust/trusted_publishers.yaml` exists, whether it parses, its version, configured publisher summaries, and findings for missing recommended fields.

The command does not verify signatures, enforce trust, mutate trust roots, install packs, download packs, execute Gadgets, or change Safe/Team/Production runtime behavior.


### Step 29 evidence and audit behavior

Step 29 is design-only. It defines the evidence and audit contract for pack trust checks, trust-root inspection, signature verification, and pack-load denials. It does not implement evidence emission, audit emission, cryptographic verification, signing tools, trust-root mutation, or enforcement.

### Step 30 diagnostic evidence and audit emission

Step 30 implements diagnostic evidence and audit emission for `gadgets pack trust check` and `gadgets pack trust roots`. Step 31 implements diagnostic evidence and audit emission for `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>`. The commands create run-scoped evidence bundles and append audit events, but they still do not verify cryptographic signatures, enforce signed-pack requirements, mutate trust roots, install packs, download packs, or execute Gadgets.

`gadgets pack trust check` emits evidence artifacts for the trust decision, pack identity, manifest hash, optional contents summary, optional signature summary, trust-root presence summary, findings, and diagnostic policy mode. It emits `pack.trust.checked` and `evidence.created` audit events.

`gadgets pack trust roots` emits evidence artifacts for the trust-root path, parsed summary, publisher summary, and findings. It emits either `trust.root.loaded` or `trust.root.missing`, plus `evidence.created`.


## Step 31 policy preview command

`gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` previews future pack-load policy outcomes without enforcing them.

Built-in packs preview as trusted in all modes. Project-local packs preview as allowed in Safe Mode with signature warnings when signatures are missing or invalid. As of Step 35, Team and Production previews consume real signature diagnostic results and allow only valid trusted signatures diagnostically. The preview command remains non-enforcing.

The command emits `pack.trust.policy.previewed` and `evidence.created` audit events and writes diagnostic evidence artifacts for the policy preview.

## Step 32 signature metadata verification scaffold

`gadgets pack trust signature [--project <path>] <pack>` validates signed-pack metadata and, as of Step 34, performs diagnostic-only Ed25519 verification when signed metadata and matching trust-root public keys are available.

The diagnostic checks:

- required `pack.signature.yaml` fields
- version `1`
- algorithm `ed25519`
- pack id and version references
- manifest hash reference
- contents hash reference when `pack.contents.yaml` is present
- strict UTC `created_at` and `expires_at` timestamp shape
- signature value presence
- matching trust-root publisher, key id, and algorithm metadata
- trust-root allowed pack id metadata

The command writes `pack.signature.checked` and `evidence.created` audit events plus diagnostic evidence artifacts. It does not enforce trust, mutate trust roots, install packs, download packs, or execute Gadgets.

## Step 33 cryptographic verification design finalization

Step 33 locks the version 1 cryptographic verification design. The first implementation must use Ed25519 signatures and SHA-256 hashes.

### Raw-byte hashes

The verifier recomputes:

```text
manifest_sha256 = sha256(raw bytes of pack.yaml)
contents_sha256 = sha256(raw bytes of pack.contents.yaml)
```

The verifier must not canonicalize YAML, reorder keys, trim whitespace, or rewrite line endings before hashing these files.

### Signature payload v1

The Ed25519 signature is over this deterministic ASCII payload encoded as UTF-8 bytes. It uses LF line endings and ends with exactly one final LF.

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

Field order is fixed. Unknown fields in `pack.signature.yaml` are not signed in v1.

### Content manifest requirements

Before a project-local pack can be trusted as signed, the verifier must check every file listed in `pack.contents.yaml`, reject duplicate paths, reject parent traversal, reject missing files, reject hash mismatches, require `pack.yaml` to be listed, and require `pack.signature.yaml` not to be listed.

### Trust-root match

The verifier must match publisher, key id, algorithm, allowed pack id, and non-expired trust-root metadata before verifying the signature.

### Rollout boundary

Step 33 did not implement cryptographic verification or enforcement. Step 34 implements Ed25519 verification for the diagnostic `gadgets pack trust signature` path only.


## Step 34 Ed25519 verification diagnostics

Step 34 implements real Ed25519 verification in `gadgets pack trust signature [--project <path>] <pack>` while keeping the command diagnostic and non-enforcing.

The diagnostic verifies:

- raw-byte SHA-256 over `pack.yaml`
- raw-byte SHA-256 over `pack.contents.yaml`
- every file hash listed in `pack.contents.yaml`
- sorted, unique, safe relative content manifest paths
- `pack.yaml` is listed in signed contents
- `pack.signature.yaml` is not listed in signed contents
- matching publisher, key id, algorithm, and allowed pack id in trust roots
- non-expired signature and trust-root metadata
- base64 Ed25519 signature over the deterministic `gadgets-pack-signature-v1` payload

The command may return `trusted_signed` as a diagnostic decision when verification succeeds. This is not pack-load enforcement. Team/Production pack-load enforcement remains deferred.

Step 34 evidence adds:

- `signature_verification_result.txt`
- `signature_payload_v1.txt`
- updated `signature_metadata_check.txt` fields for content manifest and cryptographic verification status

The evidence must not include private keys or signing seeds. Public keys are summarized by publisher, key id, algorithm, and findings rather than emitted as raw key material.

## Step 35 signature-aware policy preview

Step 35 updates `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` to consume the signature diagnostic result from `gadgets pack trust signature`.

The preview uses these inputs:

- signature metadata decision
- signature presence
- cryptographic verification performed flag
- cryptographic verification valid flag
- content manifest valid flag
- signature expiration status
- trust-root expiration status

Safe Mode remains developer-friendly and allows project-local packs with warnings when signatures are not verified. Team and Production previews allow project-local packs only when the signature diagnostic result is a valid trusted signature. These are diagnostic preview outcomes only and are not enforcement decisions.
