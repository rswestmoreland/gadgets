# Zone Model

A capability says what a Gadget may do.

A zone says where it may do it.

## Initial zones

- local_repo
- local_host_observe
- local_host_change
- staging_environment
- production_environment
- cloud_readonly
- database_readonly
- secret_handles_only

## Rule

A Gadget may only act inside its allowed zones.

Cross-zone work must happen through policy-checked handoffs.
