# Step 14 - Patch Writer Plan-Only Mode

Date: 2026-05-12

## Purpose

Step 14 adds the first Patch Writer behavior while preserving the safety boundary.

The Patch Writer can now produce a proposed patch artifact as evidence. It does not modify the working tree, apply patches, run commands, stage files, commit, open pull requests, or perform Linux administration.

## What changed

Implemented in `crates/gadgets-tools`:

- `patch_plan` module
- `PatchPlanRequest`
- `PatchPlanReport`
- `PatchPlanError`
- `run_patch_plan()`

Updated in `crates/gadgets-provider`:

- deterministic mock provider can select `patch.writer` for patch/change/test/doc/fix-style requests
- mock provider returns a structured `repo.patch.plan` handoff
- live provider system prompt now allows `patch.writer` only for plan-only patch proposals

Updated in `crates/gadgets-cli`:

- `gadgets ask <request>` can now route to either:
  - `filesystem.read` with `repo.inspect`
  - `patch.writer` with `repo.patch.plan`

Updated built-in manifest:

- `patch.writer` now declares `patch.plan`
- `patch.writer` now allowlists `patch.plan`

## Runtime path

```text
User request
  -> Coordinator provider response
      -> structured handoff to patch.writer
          -> runtime validates handoff target and task kind
              -> Patch Writer requests patch.plan action
                  -> policy evaluates manifest capability, tool, zone, and mode
                      -> proposed patch is written as evidence only
                          -> audit ledger records provider, handoff, action, evidence, and run completion
```

## Evidence artifacts

A plan-only Patch Writer run writes:

- `summary.md`
- `bundle.yaml`
- `proposed.patch`
- `patch_plan.md`
- `policy_decision.txt`
- `coordinator_plan.md` when a Coordinator handoff is present
- `assumptions.txt`

## Safety guarantees

Step 14 intentionally does not implement:

- filesystem writes
- patch application
- approval persistence
- shell execution
- test running
- Git/PR behavior
- Linux admin behavior
- database/cloud/deployment behavior

The proposed patch is a review artifact. It is not an applied change.

## Example

```bash
gadgets ask "Add parser tests"
```

Expected behavior:

- Coordinator selects Patch Writer plan-only mode.
- Runtime policy allows `patch.plan`.
- Evidence bundle includes `proposed.patch`.
- Ledger records the run.
- Working tree remains unchanged.

## Next step

Step 15 should implement approval record scaffolding for local writes before any patch application exists. The next mutating step must not apply patches until approval scope binding and exact patch hashing are represented in the runtime.
