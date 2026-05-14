# Pack Model

A pack is a named collection of Gadget manifests and supporting policy defaults.

Packs are installed into a local project through `.gadgets/config.yaml` and can be provided by built-in runtime manifests or project-local pack files.

## Developer Pack

The Developer Pack is the first implemented pack family.

Declared Gadgets include:

- Coordinator Gadget
- Filesystem Read Gadget
- Patch Writer Gadget
- Test Runner Gadget
- Git/PR Gadget
- Documentation Writer Gadget
- Secrets Guardian Gadget
- Policy Gadget
- Audit Ledger Gadget
- Approval Gadget

## Current executable Developer Pack slices

Implemented:

- Filesystem Read observe-only repo inspection
- Patch Writer plan-only proposed patch evidence
- approved local patch application through `gadgets patch apply`
- allowlisted Test Runner through `gadgets test run`
- local Git status through `gadgets git status`
- protected local branch creation through `gadgets git branch create`
- approved local commit creation through `gadgets git commit approved-patch`
- local PR body generation through `gadgets git pr body`

The Patch Writer Gadget currently supports plan-only proposed patch evidence through `patch.plan` and approved local patch application through `gadgets patch apply`. Patch application requires approval scope binding, exact patch SHA-256 verification, and path policy checks before any file write.

## Step 17 Developer Pack slice

The Test Runner Gadget manifest exists, declares `test.run`, and now has an implemented allowlisted Test Runner runtime slice.

The supported test entrypoint is:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The Test Runner runs only named commands configured in `.gadgets/config.yaml`, write evidence, and append audit events. It must not accept raw command strings, apply patches, run Git/PR actions, or perform Linux admin/database/cloud/deployment actions.

## Step 18a, Step 18b, Step 18c, and Step 19 Developer Pack slice

The Git/PR Gadget now has an implemented observe-only status slice, a protected local branch creation slice, an approved local commit slice, and a local PR body generation slice.

The supported Git entrypoint is:

```bash
gadgets git status [--project <path>]
gadgets git branch create [--project <path>] <branch-name>
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]
```

The status command runs only the fixed local status command selected by the runtime. The branch create command runs only `git branch <validated-branch-name>` after branch-name validation and protected branch checks. The approved commit command verifies a scoped patch approval, rejects protected current branches, stages only approved patch files, verifies the staged set, and creates one local commit. Local PR body generation writes evidence-only Markdown. Guarded remote PR creation can create one GitHub pull request only when explicitly enabled in config and tied to verified approval plus PR-body evidence. These slices write evidence and append audit events. They must not accept arbitrary Git arguments, checkout or switch branches, push, pull, fetch, apply patches, run tests, or perform Linux admin/database/cloud/deployment actions.

## Linux Server Admin packs

The Linux Server Admin Observe and Change packs are placeholders at the pack level. Their Gadget behaviors are not implemented yet.

Observe Pack should come before Change Pack.

Change Pack must not introduce a generic root-shell Gadget.

## Trust model

The pack trust/signing design is defined in `specs/PACK_TRUST_SIGNING_SPEC.md`.

Trust model summary:

- Pack trust is eligibility to load/use a pack, not runtime action authority.
- Built-in runtime packs are trusted by the runtime distribution.
- Project-local unsigned packs may be allowed in Safe mode only with explicit local configuration and audit warning in the future implementation.
- Team mode should require signed non-built-in packs unless an explicit team policy exception exists.
- Production mode should fail closed for unsigned, unknown, expired, mismatched, or invalid packs.
- A signed pack still cannot bypass policy, capabilities, tool allowlists, zones, approvals, evidence, or audit.

Step 26 is design-only. Step 27 adds non-enforcing `gadgets pack trust check [--project <path>] <pack>` diagnostics. Step 28 adds non-enforcing `gadgets pack trust roots [--project <path>]` diagnostics. Step 34 adds diagnostic-only Ed25519 signature verification. Step 35 makes policy preview consume those signature results. Step 37 adds a runtime dry-run gate for warning and would-deny outcomes. Step 38 adds gate preview reporting. Step 39 adds gate history reporting over audit events. Step 40 adds read-only gate status reporting over current config posture. Step 41 adds read-only gate summary reporting over trust-gate audit counts and review posture. Signing tools, trust-root mutation, registry trust, pack install/update behavior, and hard-deny enforcement are not implemented yet.


## Pack trust evidence and audit contract

Step 29 defines the evidence and audit contract for pack trust. Step 30 emits diagnostic evidence and audit for `gadgets pack trust check` and `gadgets pack trust roots`. Step 31 emits diagnostic evidence and audit for `gadgets pack trust preview`, which previews Safe/Team/Production outcomes without enforcement. Step 35 adds signature-derived policy inputs to preview evidence. Later enforcement should reuse the same decision, identity hash, signature summary, trust-root summary, policy preview, and finding concepts without copying private keys or secret-bearing config into evidence.

Pack trust now has dry-run runtime visibility for implemented Developer Pack actions. Diagnostics can verify signatures, and Step 37 can record warning or would-deny outcomes, but current runtime pack loading does not hard-deny based on signature decisions and does not add signing tools, trust-root mutation, pack install/update, or registry behavior.

## Step 32 signature metadata diagnostics

Pack trust diagnostics now include:

```bash
gadgets pack trust signature [--project <path>] <pack>
```

The command validates `pack.signature.yaml` metadata fields, local trust-root references, content manifest hashes, and Ed25519 signatures without enforcing signatures or changing pack loading behavior. Step 35 consumes these diagnostic results in `gadgets pack trust preview`.

## Step 36 effective source model

Future pack trust enforcement must evaluate the effective loaded pack material.

Effective source classes:

- `builtin`: the pack manifest and every loaded Gadget manifest come from built-in runtime assets.
- `project_local`: the pack manifest comes from project-local files.
- `project_local_mixed`: the pack manifest is built-in, but one or more Gadget manifests come from project-local overrides.
- `registry`: future installed registry pack.
- `archive`: future installed archive pack.

A `project_local_mixed` pack must not inherit built-in trust. It follows project-local trust behavior because the runtime is executing project-local manifest material.

Step 36 defines enforcement states and dry-run behavior. Step 37 implements dry-run warning and would-deny recording only. Runtime hard-deny pack-load enforcement remains unimplemented until a later approved step.

## Step 37 runtime dry-run gate

Step 37 implements effective source classification in runtime code and uses it before current Developer Pack Gadget actions execute.

A built-in pack remains effectively built-in only when every loaded Gadget manifest is also built-in. A built-in pack with a project-local Gadget override is `project_local_mixed` and follows project-local trust behavior.

Step 37 is dry-run only. It records warning or would-deny outcomes but does not add hard-deny pack-load enforcement.


## Step 39 gate history reporting

Step 39 adds a read-only command for reviewing prior pack-load trust gate outcomes:

```text
gadgets pack trust gate-history [--project <path>] [--limit <n>]
```

This command does not load additional pack material beyond reading the audit ledger, does not write evidence, and does not append audit records. It is intended for operator review before hard-deny behavior is considered.


## Step 40 gate status reporting

Step 40 adds a read-only command for reviewing current pack-load trust gate configuration:

```text
gadgets pack trust gate-status [--project <path>]
```

The command reports configured and effective Step 37 enforcement for Safe, Team, and Production. It does not inspect pack contents, verify signatures, write evidence, append audit events, or enforce hard-deny loading.

## Step 41 gate summary reporting

Step 41 adds a read-only command for summarizing current pack-load trust gate history and review posture:

```bash
gadgets pack trust gate-summary [--project <path>]
```

The command reads local config and the local audit ledger. It reports event counts for pack-load trust previews, warnings, dry-run denials, future hard-denial events, and future pack-load denial events. It does not load packs, verify signatures, write evidence, append audit events, install packs, update packs, download registry content, or enforce hard-deny.
