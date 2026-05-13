# Open Decisions

Date: 2026-05-12

## License

Status: closed.

Gadgets Framework is dual-licensed under MIT OR Apache-2.0.

Author and copyright notice:

```text
Richard S. Westmoreland
dev@rswestmore.land
Copyright 2026 Richard S. Westmoreland
```

Repository license files:

- `LICENSE.md`
- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `AUTHORS.md`
- `COPYRIGHT.md`

## Provider expansion after OpenAI and Anthropic

Status: open.

Decision so far: mock, OpenAI, and Anthropic provider adapters are implemented behind the provider trait. Additional providers may be added later only if provider output remains untrusted and runtime policy/evidence/audit remain the authority boundary.

## Approval expiration format and enforcement

Status: closed for local MVP in Step 20.

Approval expiration uses strict UTC RFC3339 without fractional seconds:

```text
YYYY-MM-DDTHH:MM:SSZ
```

Approval request creation validates the optional expiration format. Approval recording rejects expired requests. Approval verification rejects expired or malformed expiration metadata, and approval-backed workflows rely on verification before use.

Future decisions:

- whether team or production modes require expiration on every approval
- whether to support fractional seconds or full RFC3339 offsets later

## Evidence failure after mutation

Status: open.

Patch apply prepares all file changes before writing, but final evidence creation still happens after writes. A future hardening step should define how to handle evidence write failure after a successful mutation.

Possible mitigation:

- pre-create an apply-attempt evidence directory before mutation
- append a pre-write audit event
- write final evidence after mutation
- define a recovery/audit marker if final evidence cannot be completed

## Test Runner execution hardening

Status: first implementation closed; future hardening remains open.

Step 17 implements an allowlisted Test Runner, not arbitrary shell:

```bash
gadgets test run [--project <path>] <test-command-name>
```

The command string comes from `.gadgets/config.yaml`, not from provider output or free-form user input. Commands are launched without `sh -c`, shell composition syntax is rejected, stdout/stderr evidence is capped, and secret-like output lines are redacted.

Future hardening decisions:

- whether stricter modes should require a separate approval for test execution
- whether to add OS-level sandboxing or job isolation
- whether to support a richer argument parser without becoming shell-compatible
- whether to make redaction rules configurable

## Pack trust and signing

Status: deferred.

Local built-in packs and project-local pack files are enough for the current MVP. Signed packs, pack trust roots, registry behavior, and supply-chain checks remain future work.

## Git branch and commit hardening

Status: branch creation and approved local commit baselines closed; remote hardening remains open.

Step 18a implements observe-only local Git status. Step 18b implements protected local branch creation through `gadgets git branch create [--project <path>] <branch-name>`. Step 18c implements approved local commit scaffolding through `gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]`.

Closed for Step 18b:

- protected branch config shape is `git.protected_branches`
- exact entries protect exact names, and entries ending in `/` protect branch prefixes
- branch names are validated before execution
- branch creation does not require patch approval when the runtime has validated the branch name and rejected protected branches
- branch creation does not checkout, switch, stage, commit, push, fetch, open PRs, or run shell

Closed for Step 18c:

- commits bind to verified patch approval scope and the exact `proposed.patch` hash
- commit staging is limited to files named by the approved patch artifact
- preexisting staged changes are rejected
- protected current branches and detached HEAD are rejected before staging

Future decisions:

- whether commits should also require a successful named test-run evidence reference
- whether stricter modes require approval for branch creation
- how optional PR creation should be configured and gated
- whether remote PR creation is ever enabled by default


## Remote PR behavior

Closed for Step 19:

- local PR body generation writes evidence only
- `gadgets git pr body` verifies a patch approval before generation
- optional test and commit run IDs are evidence references only
- guarded GitHub PR creation is implemented only when explicitly enabled in config
- no GitHub/GitLab behavior is enabled by default
- no push, pull, fetch, merge, or rebase is performed

Still open:

- whether to add GitLab support later
- whether to allow draft PR creation as a separate config option
- whether to support fork-style head refs later
- whether remote PR creation should remain permanently manual-only
