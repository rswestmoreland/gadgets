# Step 11 - Pack Validation

Date: 2026-05-12

## Purpose

Step 11 adds validation for installed or named Gadget packs before the framework moves toward live model providers.

The goal is to make pack state visible and checkable without executing any Gadget actions.

## Implemented commands

```bash
gadgets pack validate [--project <path>] [--strict] [pack]
```

Behavior:

- With no pack name, validates all packs listed in `.gadgets/config.yaml` under `installed_packs`.
- With a pack name, validates that named pack whether it is installed or available as a built-in pack.
- Loads project-local pack manifests from `.gadgets/packs/<pack>/pack.yaml` first.
- Falls back to built-in pack manifests.
- Loads project-local Gadget manifests from `.gadgets/packs/<pack>/gadgets/<gadget>.yaml` first.
- Falls back to `.gadgets/gadgets/<gadget>.yaml`.
- Falls back to built-in Gadget manifests where implemented.
- Reports valid, missing, or invalid declared Gadget manifests.
- In default mode, missing Gadget manifests are warnings.
- In `--strict` mode, missing Gadget manifests are errors.

## Built-in Developer Pack manifests

Step 11 adds built-in manifest files for all currently declared Developer Pack Gadgets:

- `coordinator`
- `policy`
- `audit.ledger`
- `approval`
- `filesystem.read`
- `patch.writer`
- `test.runner`
- `git.pr`
- `documentation.writer`
- `secrets.guardian`

At Step 11, only `filesystem.read` was executable. Current status has advanced: `patch.writer` now supports plan-only patch evidence and approved local patch application. The remaining Developer Pack manifests are still contract placeholders.

## Validation checks

Pack validation checks:

- pack manifest can be loaded and parsed
- declared Gadget manifests can be loaded and parsed when available
- loaded Gadget `metadata.name` matches the pack declaration
- Gadget permission level does not exceed `safety.highest_permission_level`, when the pack declares one
- missing Gadget manifests are surfaced as warnings or strict errors

The core manifest parser still enforces lower-level manifest rules, including malformed capabilities, mutating capabilities without boundaries, and release capabilities without approval rules.

## Safety boundary

This step does not execute Gadgets.

It does not add:

- live OpenAI provider
- live Anthropic provider
- provider credentials
- filesystem writes
- shell execution
- patching
- test runner execution
- Git/PR behavior
- Linux admin actions
- database actions
- cloud actions

## Why this matters

Live provider support should not be added until pack and Gadget contracts are visible and consistently validated.

A model provider should only be plugged into a runtime that already understands what packs and Gadgets are installed, which manifests are available, and which declared Gadgets are still placeholders.

## Next recommended step

Implement an OpenAI provider adapter behind the existing provider trait while preserving the same runtime authority boundary:

- provider may produce structured handoff/action requests
- runtime still validates handoffs
- runtime still enforces policy
- runtime still creates evidence
- runtime still appends audit events
- provider cannot execute tools directly
