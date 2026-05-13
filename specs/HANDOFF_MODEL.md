# Handoff Model

A handoff is a structured, policy-checked request from one Gadget to another.

## Lifecycle

```text
Requesting Gadget
  -> handoff request
  -> runtime policy check
  -> target Gadget
  -> action requests
  -> runtime capability/zone/approval checks
  -> evidence
  -> audit
```

## Escalation prevention

A low-authority Gadget cannot use handoffs to indirectly gain high-authority capabilities.

The Coordinator may request many handoffs, but it cannot bypass policy.
