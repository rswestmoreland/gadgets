# Evidence Bundle Specification

Evidence bundles are structured proof packages produced by Gadgets.

A Gadget should not merely say "done." It should provide evidence showing what it inspected, changed, skipped, denied, or verified.

## Common artifacts

- summary
- file_list
- diff
- test_results
- command_log
- before_state
- after_state
- rollback_plan
- risk_assessment
- approval_record
- verification_report
- denied_access_log
- secret_scan_report

## Storage location

Evidence bundles are written under:

```text
.gadgets/runs/<run-id>/evidence/
```

The implementation hashes each artifact and stores a bundle metadata hash. It rejects unsafe run IDs and refuses to overwrite existing bundles.

## Base files

```text
bundle.yaml
summary.md
denied_actions.txt       optional
assumptions.txt
```

## Filesystem Read evidence

Observe-only filesystem runs may include:

```text
files_read.txt
skipped_paths.txt
file_excerpts.md
policy_decision.txt
coordinator_plan.md      optional
assumptions.txt
```

## Patch Writer plan evidence

Patch Writer plan-only runs may include:

```text
proposed.patch
patch_plan.md
policy_decision.txt
coordinator_plan.md      optional
assumptions.txt
summary.md
bundle.yaml
```

Patch Writer plan mode writes evidence only. It must not apply patches, stage files, commit, run tests, or open pull requests.

## Step 16 patch apply evidence

Approved local patch application produces a separate apply-run evidence bundle containing:

```text
applied.patch
files_changed.txt
before_after_hashes.txt
approval_verification.txt
policy_decisions.txt
assumptions.txt
summary.md
bundle.yaml
```

Patch apply evidence must remain separate from Patch Writer plan evidence.

## Test Runner evidence

Status: implemented at checkpoint/code level.

The allowlisted Test Runner produces a separate test-run evidence bundle containing:

```text
test_command.txt
working_dir.txt
stdout.txt
stderr.txt
exit_status.txt
duration.txt
policy_decision.txt
assumptions.txt
summary.md
bundle.yaml
```

The evidence should record whether the configured test command passed or failed. A nonzero test exit status should be recorded with stdout/stderr, not hidden as a successful run.

The Test Runner does not send stdout/stderr to a model provider. stdout/stderr evidence output is capped, and secret-like output lines are redacted through the shared best-effort redaction helper before evidence write.

## Runtime linkage

The first standalone evidence writer did not inspect files, execute commands, call providers, authorize actions, or write audit events directly. Runtime linkage now exists for Filesystem Read, Patch Writer plan-only runs, approval records, and approved local patch application. Step 17 adds linkage for allowlisted Test Runner runs. Step 18a adds linkage for local Git status runs. Step 18b adds linkage for protected local branch creation runs. Step 18c adds linkage for approved local commit runs. Step 19 adds linkage for local PR body generation runs. Step 21 adds linkage for guarded remote PR creation runs.

## Git status evidence

Step 18a Git status runs write a separate evidence bundle with these artifacts:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `git_status.txt`
- `stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `branch.txt`
- `policy_decision.txt`
- `assumptions.txt`

## Git branch creation evidence

Step 18b branch creation runs write a separate evidence bundle with these artifacts:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `branch_name.txt`
- `protected_branches.txt`
- `stdout.txt`
- `stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `policy_decision.txt`
- `assumptions.txt`

Git output is capped and secret-like lines are redacted through the shared best-effort redaction helper before evidence write.


## Git commit evidence

Step 18c approved local commit runs write a separate evidence bundle with these artifacts:

- `summary.md`
- `bundle.yaml`
- `git_command.txt`
- `approval_verification.txt`
- `approved_files.txt`
- `staged_files.txt`
- `current_branch.txt`
- `commit_message.txt`
- `commit_hash.txt`
- `git_add_stdout.txt`
- `git_add_stderr.txt`
- `git_commit_stdout.txt`
- `git_commit_stderr.txt`
- `exit_status.txt`
- `duration.txt`
- `policy_decision.txt`
- `assumptions.txt`

Git output is capped and secret-like lines are redacted through the shared best-effort redaction helper before evidence write. Commit evidence is separate from patch apply evidence and test evidence.


## PR body evidence

Step 19 local PR body generation writes a separate evidence bundle with these artifacts:

- `summary.md`
- `bundle.yaml`
- `pr_title.txt`
- `pr_body.md`
- `approval_verification.txt`
- `patch_summary.txt`
- `test_evidence.txt`
- `commit_evidence.txt`
- `policy_decision.txt`
- `assumptions.txt`

The PR body evidence bundle is separate from patch plan, patch apply, test, Git status, Git branch, Git commit, and remote PR creation evidence. It does not represent a remote PR by itself.


## Remote PR creation evidence

Step 21 guarded remote PR creation writes a separate evidence bundle with these artifacts:

- `summary.md`
- `bundle.yaml`
- `remote_pr_request.txt`
- `approval_verification.txt`
- `pr_body_reference.txt`
- `http_status.txt`
- `remote_pr_response.txt`
- `remote_pr_url.txt`
- `duration.txt`
- `policy_decision.txt`
- `assumptions.txt`

The token value loaded from the configured environment variable must never be written to evidence. The remote provider response is capped and redacted through the shared best-effort redaction helper before evidence write. Remote PR evidence is separate from PR body evidence, patch evidence, test evidence, and Git commit evidence.

## Redaction limits

Step 25 centralizes best-effort evidence redaction for stdout, stderr, Git output, local PR body text, evidence summaries, and remote API responses. The redaction helper removes whole lines containing common secret-like indicators and truncates outputs on UTF-8 boundaries.

This is not a full DLP system and must not be treated as proof that evidence is free of sensitive data. Future provider-safe summarization and stronger secret handling remain separate hardening items.

## Pack trust evidence

Step 29 defines the evidence contract for pack trust diagnostics and enforcement. Step 30 implements diagnostic evidence emission for `gadgets pack trust check` and `gadgets pack trust roots`. Step 31 implements diagnostic evidence emission for `gadgets pack trust preview`. Step 34 implements diagnostic-only Ed25519 signature verification evidence. Step 35 adds signature-derived policy inputs to preview evidence. Pack-load denial evidence remains unimplemented.

Pack trust check evidence uses:

- `summary.md`
- `bundle.yaml`
- `pack_trust_decision.txt`
- `pack_identity.yaml`
- `pack_manifest_hash.txt`
- `pack_contents_summary.txt`
- `pack_signature_summary.yaml`
- `trust_root_summary.txt`
- `trust_findings.txt`
- `policy_mode.txt`

Trust-root inspection evidence uses:

- `summary.md`
- `bundle.yaml`
- `trust_root_path.txt`
- `trust_root_summary.yaml`
- `trusted_publishers_summary.txt`
- `trust_root_findings.txt`

Future pack-load denial evidence should use:

- `summary.md`
- `bundle.yaml`
- `pack_load_denial.txt`
- `pack_trust_decision.txt`
- `trust_findings.txt`
- `requested_gadget.txt`
- `requested_capability.txt`
- `runtime_mode.txt`

Evidence must not copy private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs. Prefer key IDs, publisher names, algorithms, timestamps, and hashes over raw key material.


## Pack trust policy preview evidence

`gadgets pack trust preview` writes diagnostic-only evidence artifacts:

```text
pack_trust_policy_preview.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_trust_decision.txt
trust_findings.txt
signature_policy_inputs.txt
policy_mode.txt
```

As of Step 35, preview artifacts include signature diagnostic inputs. These artifacts preview future Safe/Team/Production pack trust outcomes. They must not be treated as enforcement evidence.

## Step 32 signature metadata diagnostic evidence

`gadgets pack trust signature [--project <path>] <pack>` writes diagnostic-only evidence artifacts:

- `summary.md`
- `bundle.yaml`
- `signature_metadata_check.txt`
- `pack_identity.yaml`
- `pack_manifest_hash.txt`
- `pack_signature_summary.yaml`
- `trust_root_summary.yaml`
- `signature_metadata_findings.txt`
- `policy_mode.txt`

These artifacts document metadata shape and trust-root reference checks. They do not prove cryptographic signature verification and do not enforce pack loading behavior.

## Step 33 future cryptographic verification evidence

Step 33 finalizes the future evidence contract for real pack signature verification.

Recommended artifacts:

```text
signature_verification_result.txt
signature_payload_v1.txt
pack_identity.yaml
pack_manifest_hash.txt
pack_contents_hash.txt
pack_contents_verification.txt
pack_signature_summary.yaml
trust_root_match.txt
signature_verification_findings.txt
policy_mode.txt
```

Evidence must not include private keys, signing seeds, API tokens, provider credentials, or full secret-bearing configs. Public key material does not need to be copied to evidence; publisher, key id, algorithm, expiration, and hashes are preferred.


## Step 34 signature verification diagnostic evidence

`gadgets pack trust signature [--project <path>] <pack>` now writes diagnostic evidence for real Ed25519 verification when signed pack metadata and matching trust-root public keys are available.

Additional or updated artifacts:

```text
signature_verification_result.txt
signature_payload_v1.txt
signature_metadata_check.txt
signature_metadata_findings.txt
```

The evidence records whether cryptographic verification was performed and whether it succeeded. It remains diagnostic only and must not be treated as pack-load enforcement evidence until Team/Production pack trust enforcement is implemented.

## Step 35 signature-aware preview evidence

`gadgets pack trust preview` now includes `signature_policy_inputs.txt`. This artifact records signature presence, cryptographic verification performed/valid status, content-manifest validity, signature expiration status, and trust-root expiration status. It remains diagnostic evidence only and must not be treated as an authoritative pack-load enforcement decision.
