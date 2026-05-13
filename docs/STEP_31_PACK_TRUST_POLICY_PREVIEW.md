# Step 31 - Pack Trust Policy Preview

Date: 2026-05-13

## Goal

Add a non-enforcing policy preview command for pack trust decisions before pack trust becomes a runtime gate.

The command is:

```bash
gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>
```

## Scope

Step 31 previews how an already-loadable pack would be treated under the Safe, Team, or Production pack-trust policy model.

The command reports:

- pack name and version
- built-in or project-local source
- diagnostic trust decision from `gadgets pack trust check`
- selected runtime mode
- future policy preview decision
- whether pack loading would be allowed
- whether a verified signature would be required
- whether a trust-root match would be required
- findings explaining the result
- evidence bundle path
- audit ledger path

## Policy preview behavior

Built-in packs are treated as trusted runtime distribution packs in Safe, Team, and Production previews.

Project-local packs are previewed as follows:

| Mode | Preview behavior |
|---|---|
| Safe | Allows unsigned local packs for developer workflows, with warnings. |
| Team | Requires verified signatures and trust-root matches. Since cryptographic verification is not implemented yet, project-local packs are reported as not loadable in the preview. |
| Production | Requires verified signatures and trust-root matches. Since cryptographic verification is not implemented yet, project-local packs are reported as not loadable in the preview. |

## Evidence artifacts

Each preview writes a diagnostic evidence bundle with:

```text
pack_trust_policy_preview.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_trust_decision.txt
trust_findings.txt
policy_mode.txt
```

## Audit events

Each preview appends:

```text
pack.trust.policy.previewed
evidence.created
```

## Non-goals

Step 31 does not add:

- cryptographic signature verification
- pack trust enforcement
- signing tools
- trust-root mutation
- pack install/update behavior
- registry downloads
- Team/Production enforcement
- Gadget execution changes
- arbitrary shell
- Linux admin, database, cloud, or deployment behavior

## Acceptance checklist

- [x] `gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>` is added.
- [x] If `--mode` is omitted, the command uses `.gadgets/config.yaml` mode.
- [x] Built-in packs preview as trusted in all modes.
- [x] Safe Mode preview allows unsigned local packs diagnostically.
- [x] Team Mode preview requires verified signatures diagnostically.
- [x] Production Mode preview requires verified signatures diagnostically.
- [x] The command writes evidence.
- [x] The command appends audit events.
- [x] The command remains non-enforcing.
- [x] No signing, install, download, trust-root mutation, or Gadget execution behavior is added.
