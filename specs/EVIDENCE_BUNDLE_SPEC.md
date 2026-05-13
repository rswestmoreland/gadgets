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

The Test Runner does not send stdout/stderr to a model provider. stdout/stderr evidence output is capped, and secret-like output lines are redacted before evidence write.

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

Git output is capped and secret-like lines are redacted before evidence write.


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

Git output is capped and secret-like lines are redacted before evidence write. Commit evidence is separate from patch apply evidence and test evidence.


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

The token value loaded from the configured environment variable must never be written to evidence. The remote provider response is capped and redacted for obvious secret-like lines before evidence write. Remote PR evidence is separate from PR body evidence, patch evidence, test evidence, and Git commit evidence.
