# Gadget Manifest Specification

A Gadget manifest is the static declaration of what a Gadget is, what it can do, where it can act, what evidence it must produce, and which other Gadgets it may contact.

The manifest is not a prompt. It is a runtime-enforced contract.

## Minimal example

```yaml
schema_version: gadgets.framework/v0.1
kind: Gadget

metadata:
  name: filesystem.read
  version: 0.1.0
  display_name: Filesystem Read Gadget
  description: Reads scoped project files and produces summaries.

runtime:
  model_profile: general_default
  execution_mode: observe

permission_level: observe

capabilities:
  - repo.read
  - file.read
  - file.search

boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable: false
    denied_paths:
      - ".git/"
      - ".env"
      - "secrets/"
      - "**/*secret*"
      - "**/*credential*"

tools:
  allowed:
    - file.list
    - file.read
    - file.search

handoffs:
  allowed_targets:
    - documentation.writer
    - patch.writer
    - test.runner

evidence:
  required:
    - summary
    - files_read
    - denied_accesses
    - assumptions

approval:
  required_for: []
```

## Validation rules

Reject manifests when:

- required metadata is missing
- permission level is invalid
- mutating capability lacks boundaries
- release-level capability lacks approval rules
- requested capability is unknown
- requested tool is not installed
- handoff target is unknown
