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

Pack signing, registry trust, and external pack supply-chain controls are deferred.
