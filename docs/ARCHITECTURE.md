# Architecture

Date: 2026-05-13

## Purpose

This document describes the current architecture for Gadgets Framework after Step 20 local Developer MVP hardening.

## Concept

Gadgets Framework separates reasoning from authority.

A model can understand a request, propose a plan, and request a tool action. The runtime decides whether that action is allowed.

Provider SDK behavior, prompts, model tool-calling, and agent handoff features are integration surfaces. They are not final security boundaries.

## Runtime authority path

```text
User
  -> Coordinator Gadget
      -> structured handoff request
          -> runtime policy check
              -> target Gadget
                  -> structured action request
                      -> runtime capability/zone/approval check
                          -> tool/action provider
                              -> evidence bundle
                                  -> audit ledger
```

## Main components

### CLI

The first user interface.

Current implemented commands:

```bash
gadgets init [path]
gadgets ask [--project <path>] <request>
gadgets ledger show [project-root-or-ledger-path]
gadgets ledger verify [project-root-or-ledger-path]
gadgets evidence show <run-id> [project-root]
gadgets evidence verify <run-id> [project-root]
gadgets evidence create-observe <run-id> <gadget> <summary>
gadgets pack list [--project <path>]
gadgets pack show [--project <path>] <pack>
gadgets pack validate [--project <path>] [--strict] [pack]
gadgets pack trust check [--project <path>] <pack>
gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]
gadgets approval approve [--project <path>] <approval-request-id> <approver>
gadgets approval show [--project <path>] <approval-request-id>
gadgets approval verify [--project <path>] <approval-request-id>
gadgets approval id-for-run <run-id>
gadgets patch apply [--project <path>] <approval-request-id>
gadgets test run [--project <path>] <test-command-name>
gadgets git status [--project <path>]
gadgets git branch create [--project <path>] <branch-name>
gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]
```

The Test Runner command executes only named commands configured in `.gadgets/config.yaml`. It does not accept arbitrary command strings from the model or from free-form user requests. The Git status command runs only one fixed local observe command selected by the runtime. The Git branch create command runs only one fixed local branch command after runtime branch-name validation and protected-branch checks. The approved commit command verifies a scoped patch approval, rejects protected current branches, stages only approved patch files, and creates one local commit. The PR body command verifies approval and evidence references, then writes local Markdown evidence only.

### Core runtime

Owns:

- run lifecycle
- Gadget loading
- pack loading
- handoff routing
- action authorization
- evidence creation
- audit recording

### Policy engine

The first deterministic built-in policy engine is implemented in `crates/gadgets-policy`.

Checks include:

- Gadget identity
- permission level
- manifest capability
- active mode
- target zone
- resource boundary
- approval requirement
- evidence requirement
- denied paths and resources

Step 17 adds a deterministic `test.run` policy path for named commands loaded from the local config allowlist before execution. Step 18a adds a deterministic `git.status` policy path for fixed local Git status before execution. Step 18b adds a deterministic `git.branch.create` policy path for validated, non-protected local branch creation before execution. Step 18c adds a deterministic `git.commit.create` policy path for verified approval-backed commits that stage only approved patch files. Step 19 adds a deterministic `git.pr.body.generate` policy path for local PR body evidence generation. Step 20 enforces approval expiration before approval-backed workflows can use an approval.

### Provider adapters

Model providers are adapters, not authority boundaries. `.gadgets/config.yaml` selects a model profile, and the runtime chooses the matching provider adapter.

Initial provider set:

1. Mock provider. Status: implemented and default.
2. OpenAI provider. Status: implemented behind provider trait.
3. Anthropic provider. Status: implemented behind provider trait.

Provider output is treated as untrusted structured input. Providers cannot execute tools, approve actions, mutate files, run tests, access secrets directly, or bypass policy/evidence/audit.

### Evidence writer

The evidence writer persists structured proof bundles for Gadget runs. The implementation supports YAML metadata, Markdown summaries, artifact hashes, and bundle verification.

The evidence writer does not authorize or execute actions. It records caller-supplied evidence after runtime policy decisions are made.

Test Runner, Git status, Git branch, Git commit, and PR body evidence remain separate from patch apply evidence. A test run produces its own run directory and artifacts such as stdout, stderr, exit status, duration, policy decision, command name, working directory, assumptions, and summary. A branch creation run produces its own artifacts for the fixed Git command, branch name, protected branch config, stdout, stderr, exit status, duration, policy decision, assumptions, and summary. A commit run produces its own artifacts for approval verification, approved files, staged files, current branch, commit message, commit hash, Git stdout/stderr, exit status, duration, policy decision, assumptions, and summary. A PR body run produces its own artifacts for PR title, PR body Markdown, approval verification, patch summary, optional test evidence reference, optional commit evidence reference, policy decision, assumptions, and summary.

### Pack and Gadget manifest loader

The CLI loads installed pack names from `.gadgets/config.yaml`. Pack manifests can come from project-local `.gadgets/packs/` overrides or built-in manifests. Gadget manifests can come from project-local pack Gadget directories, project-global `.gadgets/gadgets/`, or built-in Gadget manifests when implemented.

The current `ask` flow requires the Developer Pack and loads `filesystem.read` or `patch.writer` through this manifest loader depending on the Coordinator handoff.

### Tool/action providers

Execution happens through narrow providers.

Implemented providers and runtime slices:

- filesystem read provider
- Patch Writer plan-only provider
- approved local patch apply provider
- allowlisted test command runner
- local Git status provider
- protected local Git branch provider
- approved local Git commit provider
- local PR body generator
- audit ledger writer
- evidence bundle writer
- mock model provider
- OpenAI model provider adapter
- Anthropic model provider adapter

Deferred providers:

- process inspector
- service manager
- firewall
- package manager
- container runtime
- database
- cloud

## Current local Developer Pack runtime slices

The current implementation includes `gadgets ask [--project <path>] <request>` for local repository inspection and plan-only Patch Writer requests. The command loads `.gadgets/config.yaml`, selects the configured provider profile, receives a structured Coordinator handoff, loads the relevant Developer Pack Gadget manifest, evaluates actions through policy, writes evidence bundles, and appends audit ledger events.

Filesystem Read remains observe-only and modifies no files. Patch Writer plan mode writes only evidence. Approved local patch application is separate and only runs through `gadgets patch apply` after approval record, scope hash, patch SHA-256, and writable path policy checks all pass.

Patch apply does not run tests or Git commands. Test running is a separate explicit CLI path using only configured command names. Git status, local branch creation, approved local commit creation, PR body generation, and guarded remote PR creation are separate explicit CLI paths. Git commit does not apply patches; it can only stage and commit files named by a verified approved patch artifact. PR body generation writes reviewable local Markdown evidence only. Remote PR creation is disabled by default and requires explicit config, verified approval, PR body evidence, configured branch constraints, duplicate-open-PR handling, and dry-run/execute config. Generated config keeps dry-run mode enabled by default.

## Step 17 Test Runner boundary

The allowlisted Test Runner is implemented as a local Developer Pack tool/action provider.

Required boundary:

- `gadgets test run [--project <path>] <test-command-name>` is the first supported entrypoint.
- The command string must come only from `.gadgets/config.yaml`.
- The model must not supply a raw command string.
- User prompts must not supply a raw command string.
- Unknown command names must be rejected.
- Empty command names must be rejected.
- Working directories must stay inside the project boundary.
- Parent traversal in `working_dir` must be rejected.
- Runtime policy must check `test.run` before execution.
- stdout, stderr, exit status, duration, and pass/fail are captured as evidence.
- Evidence and audit must be written for meaningful work.
- Test execution must not apply patches, stage files, commit, create PRs, or perform Linux admin actions.
- Commands are launched directly without `sh -c`; configured commands containing shell composition syntax are rejected.

## Step 25 redaction boundary

Evidence output redaction is centralized in `gadgets-tools`. Current providers use a shared best-effort helper for Test Runner stdout/stderr, Git command output, local PR body text, referenced evidence summaries, and remote PR API responses. The helper redacts whole lines containing common secret-like indicators and truncates captured output on UTF-8 boundaries.

This is not full DLP or a guarantee that evidence is free of sensitive data. Raw evidence must still be handled as potentially sensitive, and provider-safe evidence summarization remains future work.

## Safety defaults

- Safe Mode default.
- Unknown capabilities denied.
- Production denied by default.
- Secret values never passed to models.
- File writes require approval in v0.1; protected local branch creation is a narrow validated Git ref action with its own policy context; approved local commits require verified patch approval scope.
- Patch Writer plan mode may produce evidence-only proposed diffs.
- Approved local patch application is available only through `gadgets patch apply` after approval record, scope hash, patch hash, and path policy verification.
- No arbitrary shell in the MVP.
- Test execution is allowlisted and named; it is not generic shell.
- Audit/evidence required for meaningful work.


## Step 26 pack trust boundary

Pack trust is a supply-chain eligibility check. It answers whether a pack is allowed to be loaded and targeted by the runtime. It does not grant the pack authority to bypass policy.

The Step 26 design defines pack identity, content manifests, detached signature records, trust roots, Safe/Team/Production mode behavior, verification outcomes, and future audit/evidence expectations. The contract is in `specs/PACK_TRUST_SIGNING_SPEC.md`.

A trusted signed pack still must pass Gadget manifest validation, capability checks, tool allowlist checks, zone and path boundary checks, runtime mode restrictions, approval requirements, evidence creation, and audit logging. Provider output remains untrusted even when it targets a trusted pack.

Step 27 adds non-enforcing diagnostics through `gadgets pack trust check [--project <path>] <pack>`. The command reports built-in/project-local status, optional content/signature metadata, basic metadata hash cross-checks, and trust-root-file presence.

Step 28 adds `gadgets pack trust roots [--project <path>]` as a non-mutating trust-root inspection command. It reports whether `.gadgets/trust/trusted_publishers.yaml` exists, whether it parses, its version, publisher summaries, and findings. These commands do not verify cryptographic signatures, enforce signed-pack requirements, mutate trust roots, install packs, download packs, or execute Gadgets.

Step 29 defines the evidence and audit contract for pack trust decisions, trust-root loading, signature verification, and pack-load denials. Step 30 emits diagnostic evidence bundles and audit events for `gadgets pack trust check` and `gadgets pack trust roots`. Step 31 adds `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` to preview future Safe/Team/Production policy outcomes without enforcing them. Step 34 adds non-enforcing Ed25519 verification diagnostics to `gadgets pack trust signature`. Step 35 makes policy preview consume the real signature diagnostic result while remaining non-enforcing. Pack trust enforcement is not implemented yet.

## Pack validation boundary

Pack validation is a pre-execution contract check. It confirms which packs are installed, where manifests are loaded from, which declared Gadgets are available, and whether any manifests are invalid or pending. Validation does not execute Gadgets or call model providers.

## Live provider boundary

OpenAI and Anthropic adapters can produce Coordinator text and structured handoff requests, but they cannot execute tools. Runtime policy, evidence, and audit remain the authority path for every action.

## Approval scaffolding

Approval records live under `.gadgets/approvals/`. A patch approval request binds a future `repo.patch.apply` operation to a specific Patch Writer run and the SHA-256 hash of its `proposed.patch` artifact. Approval requests may include strict UTC expiration in the form `YYYY-MM-DDTHH:MM:SSZ`. Approval records are evidence of human approval, not execution by themselves. Future execution must still pass approval verification, expiration checks, policy checks, and path-boundary checks.

## Local Git boundary

`gadgets git status [--project <path>]` runs only the fixed local command `git status --short --branch --untracked-files=normal`. It is selected by the runtime, launched without `sh -c`, checked by policy as `git.status`, and recorded with evidence and audit.

`gadgets git branch create [--project <path>] <branch-name>` runs only the fixed local command `git branch <validated-branch-name>`. The branch name is passed as a single process argument after runtime validation. Protected branch names are loaded from `.gadgets/config.yaml` under `git.protected_branches`; exact names such as `main` and prefix entries such as `release/` are denied before execution.

Branch creation does not checkout or switch branches, stage files, commit, push, pull, fetch, merge, rebase, create PRs, call providers, apply patches, run tests, or perform admin actions.

`gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]` verifies the approval request and approval record, verifies the exact patch artifact hash and scope hash, extracts approved file paths from `proposed.patch`, rejects detached HEAD and protected current branches, rejects preexisting staged changes, stages only approved patch files, verifies the staged set, and creates one local commit. It uses fixed Git commands through `std::process::Command` without `sh -c`. If commit creation fails after staging, it attempts a best-effort fixed `git reset -- <approved-files>` cleanup.

Commit creation does not checkout or switch branches, push, pull, fetch, merge, rebase, create PRs, call providers, apply patches, run tests, or perform admin actions.

`gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]` verifies the approval request and approval record, summarizes the approved patch, optionally references test and commit evidence bundles, and writes PR title/body artifacts to a separate evidence bundle. `gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]` can create one GitHub pull request only when `git.remote_pr.enabled` is true, `git.remote_pr.dry_run` is false, branch allowlists pass, duplicate-open-PR checks pass, and the approval plus PR-body evidence are verified. With dry-run enabled, it writes evidence without making the GitHub mutation call. It does not push, pull, fetch, merge, rebase, checkout, switch, call model providers, apply patches, run tests, execute shell, or perform admin actions.

## Step 32 signature metadata diagnostics

The pack trust diagnostic layer includes `gadgets pack trust signature [--project <path>] <pack>`. This command checks signature metadata shape and trust-root references, validates content manifest file hashes, verifies Ed25519 signatures when signed metadata and trust-root public keys are available, writes evidence, and appends audit events. It remains outside runtime enforcement: no pack-load blocking, trust-root mutation, signing, install/update, downloads, or Gadget execution behavior is added.

## Step 33 cryptographic verification design boundary

Step 33 locks the architecture for real pack cryptographic verification without implementing it.

The architecture now treats a signed project-local pack as four inputs:

- `pack.yaml`
- `pack.contents.yaml`
- `pack.signature.yaml`
- `.gadgets/trust/trusted_publishers.yaml`

The diagnostic verification flow recomputes SHA-256 over the raw bytes of `pack.yaml` and `pack.contents.yaml`, validates every file hash listed in the content manifest, matches publisher/key/algorithm/allowed pack id against the trust root, and verifies an Ed25519 signature over a deterministic line-based `gadgets-pack-signature-v1` payload. The result is diagnostic evidence, not pack-load enforcement.

This is still outside enforcement. Pack loading, Gadget execution, Team/Production enforcement, signing tools, trust-root mutation, and registry behavior are unchanged by Step 33.


## Step 34 Ed25519 verification diagnostics

Step 34 implements real signature verification in the diagnostic path only. The runtime still does not enforce pack trust during pack loading.

The diagnostic path verifies:

- `pack.yaml` raw-byte SHA-256
- `pack.contents.yaml` raw-byte SHA-256
- listed file hashes and safe content paths
- signature and trust-root expiration metadata
- publisher, key id, algorithm, and allowed pack id trust-root match
- Ed25519 signature over the deterministic `gadgets-pack-signature-v1` payload

This creates an observable verification result before any future Team/Production enforcement is added.

## Step 35 signature-aware policy preview

Step 35 updates the non-enforcing pack trust policy preview so it consumes the same signature metadata, content manifest, trust-root, expiration, and Ed25519 verification results produced by `gadgets pack trust signature`.

Safe Mode remains developer-friendly and previews local packs as allowed with warnings when signatures are not verified. Team and Production previews allow only valid trusted signatures diagnostically. Runtime pack loading is not enforced by this step.
