# Decision Record

Date: 2026-05-12

## DR-001 - Rust core runtime

Decision: Use Rust for the safety-critical runtime.

Reason: The runtime enforces authority boundaries, policy, evidence, audit, and provider/tool isolation.

## DR-002 - CLI-first MVP

Decision: Start with a local CLI.

Reason: A CLI proves the safety model without requiring a service platform, UI, or distributed deployment.

## DR-003 - Provider-neutral model layer

Decision: Use provider adapters.

Reason: OpenAI, Anthropic, local models, and future providers should be interchangeable behind the Gadget runtime.

## DR-004 - Mock provider first

Decision: Implement mock provider before a live provider.

Reason: The runtime and policy model must be testable without live model behavior.

## DR-005 - YAML manifests and JSONL audit

Decision: Use YAML for human-authored manifests/config and JSONL for append-only event streams.

Reason: YAML is readable. JSONL is simple, appendable, and easy to verify.

## DR-006 - Built-in policy first

Decision: Use deterministic built-in policy checks before policy-as-code.

Reason: Avoid Kubernetes-like complexity in the first release.

## DR-007 - Safe Mode default

Decision: Safe Mode is default.

Reason: The default experience must prevent production writes, destructive actions, and secret exposure.

## DR-008 - Developer Pack first

Decision: Build Developer Pack before server admin, database, cloud, or deployment packs.

Reason: Developer automation provides useful workflows with lower blast radius.

## DR-009 - Linux Server Admin as pack family

Decision: Add Linux Server Admin Observe Pack before Change Pack.

Reason: Server administration is powerful and common, but mutation must be tightly gated.

## DR-010 - No generic root-shell Gadget

Decision: Do not build a broad shell/root Gadget.

Reason: It undermines the entire least-privilege Gadget model.

## DR-011 - Approval required for file writes in v0.1

Decision: All writes require explicit approval in the first release.

Reason: This is the simplest and safest default.

## DR-012 - Markdown docs always maintained

Decision: Every meaningful build/design step updates Markdown documentation.

Reason: Docs preserve design continuity and can later support user-facing and marketing material.

## License

Decision: Gadgets Framework is dual-licensed under MIT OR Apache-2.0.

Author: Richard S. Westmoreland <dev@rswestmore.land>

Copyright 2026 Richard S. Westmoreland
