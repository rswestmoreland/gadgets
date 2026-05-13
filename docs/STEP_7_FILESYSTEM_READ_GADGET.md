# Step 7 - Observe-only Filesystem Read Gadget

Date: 2026-05-12

## Purpose

Step 7 implements the first real observe-only Gadget slice.

The Filesystem Read Gadget can inspect a local project directory, but every directory traversal and file read is evaluated through the built-in policy engine before access. The run produces an evidence bundle and appends audit ledger events.

## Implemented behavior

New user command:

```bash
gadgets ask "Review this repo and explain how it is structured."
```

The command currently uses a deterministic local flow rather than a live model provider.

Runtime behavior:

1. Creates a run id.
2. Loads the built-in `filesystem.read` Gadget manifest.
3. Records `run.started` and `handoff.allowed` audit events.
4. Traverses the local repository in observe-only mode.
5. Evaluates candidate directories using `file.search` through policy.
6. Evaluates candidate files using `file.read` through policy.
7. Records allowed and denied action decisions in the audit ledger.
8. Reads bounded file excerpts only for allowed paths.
9. Writes an evidence bundle under `.gadgets/runs/<run-id>/evidence/`.
10. Records `evidence.created` and `run.completed` audit events.

## Safety properties

This step does not:

- write project files
- execute shell commands
- call model providers
- run tests
- apply patches
- use Git
- perform Linux administration
- access denied secret/protected paths

Denied paths include:

- `.git/`
- `.gadgets/`
- `.env`
- `secrets/`
- `**/*.pem`
- `**/*.key`
- `**/*secret*`
- `**/*credential*`

## Evidence artifacts

The evidence bundle now includes:

- `summary.md`
- `bundle.yaml`
- `denied_actions.txt` when denied paths/actions exist
- `assumptions.txt`
- `files_read.txt`
- `skipped_paths.txt`
- `file_excerpts.md`

Evidence verification now checks both:

1. bundle metadata hash
2. artifact file hashes

This means changes to evidence artifacts are detected by `gadgets evidence verify <run-id>`.

## Audit events

The ledger records events such as:

- `run.started`
- `handoff.allowed`
- `action.allowed`
- `action.denied`
- `evidence.created`
- `run.completed`

The existing hash-chain ledger verification remains unchanged.

## CLI examples

Initialize local state:

```bash
gadgets init
```

Run observe-only repository inspection:

```bash
gadgets ask "Review this repo and explain how it is structured."
```

Show audit history:

```bash
gadgets ledger show
```

Verify audit history:

```bash
gadgets ledger verify
```

Show evidence:

```bash
gadgets evidence show <run-id>
```

Verify evidence:

```bash
gadgets evidence verify <run-id>
```

## Acceptance status

Implemented in this checkpoint:

- observe-only Filesystem Read provider
- policy enforcement for traversal and reads
- bounded file excerpts
- denied path recording
- evidence bundle creation
- audit event recording
- CLI `ask` command for the first vertical slice
- artifact hash verification for evidence bundles

Still deferred:

- live model provider calls
- real Coordinator reasoning
- patch planning
- approved patch application
- test runner
- Git/PR workflow
- Linux Server Admin Observe Pack
- Linux Server Admin Change Pack
