//! Core contracts for Gadgets Framework.
//!
//! This crate contains the first implementation of the Phase 0 contract model:
//! manifests, packs, capabilities, permissions, zones, handoffs, actions,
//! policy decisions, evidence metadata, audit events, and validation errors.
//!
//! This crate does not execute tools or call model providers. It only parses and
//! validates contract data.

pub mod action;
pub mod audit;
pub mod capability;
pub mod error;
pub mod evidence;
pub mod handoff;
pub mod manifest;
pub mod pack;
pub mod permission;
pub mod policy;
pub mod validation;
pub mod zone;

pub use action::{ActionRequest, ActionTarget};
pub use audit::{AuditActor, AuditEvent, AuditTarget};
pub use capability::CapabilityName;
pub use error::{GadgetCoreError, ValidationError};
pub use evidence::{EvidenceArtifact, EvidenceBundle};
pub use handoff::{HandoffRequest, HandoffScope};
pub use manifest::{GadgetManifest, ManifestKind};
pub use pack::PackManifest;
pub use permission::PermissionLevel;
pub use policy::{DecisionKind, PolicyDecision};
pub use validation::{Validate, ValidationReport};
pub use zone::{BoundarySet, FilesystemBoundary, ZoneRef};
