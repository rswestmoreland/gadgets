# Step 6 - Built-in Deterministic Policy Engine

Date: 2026-05-12

## Purpose

Step 6 adds the first deterministic policy engine for Gadgets Framework.

This step still does not execute tools, call model providers, inspect files, write patches, run tests, perform shell commands, or administer Linux hosts. It only evaluates structured action requests against a Gadget manifest and runtime policy context.

## Implemented crate

`crates/gadgets-policy`

## Core rule

A model or Gadget may request an action. The runtime policy engine decides whether the action is allowed, denied, or requires approval.

## Implemented checks

The policy engine now evaluates:

- requesting Gadget identity matches the manifest
- requested capability is declared by the Gadget
- requested capability does not exceed the Gadget permission level
- requested tool is allowlisted by the Gadget manifest
- target zone is present and allowed by the Gadget boundary
- filesystem actions include a path
- filesystem paths are relative and do not traverse parent directories
- filesystem paths do not match denied paths
- filesystem paths are inside configured roots
- read paths honor readable paths when configured
- write paths honor writable paths when configured
- Safe Mode blocks release-level actions
- Team Mode blocks release-level actions without a production gate
- mutating actions require approval before execution

## Decision results

The engine returns a `PolicyDecision` with one of:

- `allowed`
- `denied`
- `requires_approval`

The engine also returns internal findings for tests and debugging.

## Conservative default

The implementation is intentionally conservative.

Denied examples:

- unknown or undeclared capability
- tool not allowlisted
- missing target zone
- zone not allowed
- path traversal such as `../secret.txt`
- denied paths such as `.env`, `.git/`, `secrets/`, private keys, or secret-like names
- writes outside writable paths
- release-level actions in Safe Mode

Approval-required example:

- approved-capable local write inside writable paths, but no approval is present

## Tests added

Unit tests cover:

- allowed read inside allowed zone/path
- path traversal denial
- denied secret/protected paths
- missing capability denial
- tool-not-allowed denial
- zone denial
- write requires approval
- approved write inside writable path
- approved write outside writable path denial
- Safe Mode blocks release actions
- Production Mode release action requires approval

## Not implemented yet

- approval persistence
- approval scope hashes
- policy records in the audit ledger
- evidence linkage for policy decisions
- config loading from `.gadgets/config.yaml`
- filesystem inspection
- model/provider integration
- patch application
- allowlisted test execution was not part of this historical Step 6 checkpoint
- Linux admin actions

## Next step

Step 7 should implement the observe-only Filesystem Read Gadget and wire it through policy, evidence, and audit in a controlled runtime path.
