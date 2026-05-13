# Step 10 - Pack and Gadget Manifest Loading

Date: 2026-05-12

## Purpose

Step 10 moves the CLI away from hardcoded runtime manifests and toward the pack/Gadget loading model defined in Phase 0.

At Step 10, the runtime still executed only the observe-only Filesystem Read slice, but it started loading the Developer Pack and Filesystem Read Gadget manifest through a manifest loader rather than embedding that decision directly in `gadgets ask`. Current status has advanced: Patch Writer plan/apply behavior now also uses the manifest-loaded Developer Pack.

## Implemented

- Added `crates/gadgets-cli/src/manifest_loader.rs`.
- Loads installed pack names from `.gadgets/config.yaml`.
- Loads project pack manifests from `.gadgets/packs/<pack>/pack.yaml` when present.
- Falls back to built-in pack manifests when no local override exists.
- Loads project Gadget manifests from:
  - `.gadgets/packs/<pack>/gadgets/<gadget>.yaml`
  - `.gadgets/gadgets/<gadget>.yaml`
- Falls back to the built-in Developer Pack `filesystem.read` Gadget manifest.
- Verifies that the Developer Pack is installed before `gadgets ask` can run the current observe workflow.
- Prints the pack source and Gadget manifest source during `gadgets ask`.
- Added `gadgets pack list [--project <path>]`.
- Added `gadgets pack show [--project <path>] <pack>`.

## Runtime behavior

Current `gadgets ask` flow:

```text
load .gadgets/config.yaml
  -> select provider profile
  -> verify Developer Pack is installed
  -> load Developer Pack manifest
  -> load filesystem.read Gadget manifest
  -> call deterministic mock Coordinator
  -> validate coordinator handoff
  -> run Filesystem Read through policy/evidence/audit
```

The model/provider still does not execute actions directly.

## Pack source order

Pack manifests:

1. `.gadgets/packs/<pack>/pack.yaml`
2. built-in pack manifest

Gadget manifests:

1. `.gadgets/packs/<pack>/gadgets/<gadget>.yaml`
2. `.gadgets/gadgets/<gadget>.yaml`
3. built-in Gadget manifest, when implemented

## Current built-in manifest coverage

Built-in pack manifests exist for:

- `developer`
- `linux-admin-observe`
- `linux-admin-change`

At this step, only the Developer Pack `filesystem.read` Gadget had an executable built-in Gadget manifest. Current status has advanced: `patch.writer` now supports plan-only patch evidence and approved local patch application.

Other declared Gadgets intentionally show as `manifest pending` in `gadgets pack show` until their implementation steps arrive.

## New CLI commands

```bash
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
```

## Not implemented yet

- Installed pack copying during `gadgets init`.
- Public pack registry.
- Pack signing/trust model.
- Live OpenAI or Anthropic providers.
- Patch Writer, Test Runner, Git/PR, or Linux admin Gadget execution.
- At Step 10, runtime execution of any Gadget except observe-only Filesystem Read was still deferred. Current status has advanced through approved local Patch Writer application.

## Next step

Implement the OpenAI provider adapter skeleton behind the existing provider trait, or continue hardening the local manifest/runtime flow by adding a `gadgets pack validate` command.

Recommendation: add `gadgets pack validate` before live providers, so pack and Gadget manifest problems can be found without running a workflow.
