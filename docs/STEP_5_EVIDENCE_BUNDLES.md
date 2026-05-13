# Step 5 - Evidence Bundle Writer

Date: 2026-05-12

## Purpose

Step 5 adds the first evidence persistence layer for Gadgets Framework.

The evidence layer records structured proof of Gadget work. A Gadget should not merely say "done." It should produce a bundle that explains what happened, which artifacts were produced, which actions were denied, which assumptions were made, and how the bundle can be verified.

## Scope

Implemented in this checkpoint:

- `crates/gadgets-evidence`
- observe-only evidence bundle creation
- bundle metadata written as YAML
- human summary written as Markdown
- optional denied-action and assumption artifacts
- SHA-256 artifact hashes
- bundle metadata hash
- bundle read, summarize, and verify helpers
- CLI evidence commands

## CLI commands

```bash
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
```

`create-observe` is a development helper. It does not inspect files, call models, execute commands, or authorize actions. It only writes an observe-only evidence bundle from caller-supplied data.

## Evidence location

Evidence is stored under:

```text
.gadgets/runs/<run-id>/evidence/
```

An observe-only bundle currently writes:

```text
bundle.yaml
summary.md
denied_actions.txt       optional
assumptions.txt          optional
```

## Safety behavior

The evidence writer:

- rejects unsafe run IDs and Gadget IDs
- refuses to overwrite an existing bundle
- hashes artifacts
- hashes bundle metadata
- distinguishes evidence persistence from action execution

The evidence writer does not:

- inspect project files
- execute shell commands
- call model providers
- authorize actions
- apply patches
- persist audit events directly
- perform Linux administration

## Relationship to the audit ledger

The audit ledger and evidence writer are separate layers.

- The ledger records event history.
- Evidence bundles store proof artifacts for a run.

A future runtime step will link ledger events to evidence bundle IDs during actual Gadget runs.

## Acceptance status

Step 5 is complete when the repository contains:

- evidence crate implementation
- CLI show/verify/create-observe commands
- docs/spec updates
- tests for write/read/verify/overwrite/path traversal/tamper cases

No runtime Gadget execution is introduced in this step.
