//! Plan-only Patch Writer provider.
//!
//! This module implements the first Patch Writer slice. It can create a
//! proposed patch artifact as evidence, but it never modifies files, runs
//! commands, stages changes, or applies a diff. The proposed patch is a draft
//! artifact for human review and later approved execution.

use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    create_observe_bundle, default_runs_root, EvidenceTextArtifact, EvidenceWriteRequest,
};
use gadgets_ledger::{append_event, default_ledger_path, new_audit_event, with_target, LedgerError};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

const DEFAULT_ZONE: &str = "local_repo";
const PATCH_PLAN_CAPABILITY: &str = "patch.plan";
const PATCH_PLAN_TOOL: &str = "patch.plan";

#[derive(Debug)]
pub enum PatchPlanError {
    Io(std::io::Error),
    Ledger(LedgerError),
    Evidence(gadgets_evidence::EvidenceError),
    Capability(String),
    PolicyDenied(String),
    InvalidProjectRoot(PathBuf),
}

impl fmt::Display for PatchPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "patch plan I/O error: {err}"),
            Self::Ledger(err) => write!(f, "patch plan ledger error: {err}"),
            Self::Evidence(err) => write!(f, "patch plan evidence error: {err}"),
            Self::Capability(err) => write!(f, "patch plan capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "patch plan denied by policy: {reason}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
        }
    }
}

impl Error for PatchPlanError {}

impl From<std::io::Error> for PatchPlanError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<LedgerError> for PatchPlanError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<gadgets_evidence::EvidenceError> for PatchPlanError {
    fn from(value: gadgets_evidence::EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlanRequest {
    pub run_id: String,
    pub created_at: String,
    pub user_prompt: String,
    pub zone: String,
    pub coordinator_summary: Option<String>,
    pub handoff_id: Option<String>,
    pub handoff_reason: Option<String>,
    pub provider_name: Option<String>,
    pub runtime_mode: RuntimeMode,
}

impl PatchPlanRequest {
    pub fn plan_patch(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        user_prompt: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            user_prompt: user_prompt.into(),
            zone: DEFAULT_ZONE.to_string(),
            coordinator_summary: None,
            handoff_id: None,
            handoff_reason: None,
            provider_name: None,
            runtime_mode: RuntimeMode::Safe,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }

    pub fn with_coordinator_handoff(
        mut self,
        coordinator_summary: impl Into<String>,
        handoff_id: impl Into<String>,
        handoff_reason: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        self.coordinator_summary = Some(coordinator_summary.into());
        self.handoff_id = Some(handoff_id.into());
        self.handoff_reason = Some(handoff_reason.into());
        self.provider_name = Some(provider_name.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlanReport {
    pub run_id: String,
    pub policy_decision: DecisionKind,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct PlanState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

pub fn run_patch_plan(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: PatchPlanRequest,
) -> Result<PatchPlanReport, PatchPlanError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(PatchPlanError::InvalidProjectRoot(project_root.to_path_buf()));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = PlanState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.started",
        "gadget",
        "coordinator",
        None,
        "allowed",
        "Plan-only Patch Writer run started.",
    )?;

    if let Some(summary) = request.coordinator_summary.as_deref() {
        let provider_name = request.provider_name.as_deref().unwrap_or("mock");
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "provider.response",
            "provider",
            provider_name,
            Some(("gadget", "coordinator")),
            "allowed",
            summary,
        )?;
    }

    if let Some(handoff_id) = request.handoff_id.as_deref() {
        let reason = request
            .handoff_reason
            .as_deref()
            .unwrap_or("Coordinator requested plan-only patch proposal.");
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "handoff.requested",
            "gadget",
            "coordinator",
            Some(("handoff", handoff_id)),
            "allowed",
            reason,
        )?;
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "handoff.allowed",
        "gadget",
        "coordinator",
        Some(("gadget", &gadget.metadata.name)),
        "allowed",
        "Coordinator handed off plan-only patch proposal to Patch Writer Gadget.",
    )?;

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(PATCH_PLAN_CAPABILITY)
            .map_err(|err| PatchPlanError::Capability(err.to_string()))?,
        tool: PATCH_PLAN_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: None,
            resource: Some("proposed.patch".to_string()),
        },
        reason: "Create a plan-only proposed patch artifact as evidence.".to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: false,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: false,
        approved_git_commit: false,
        approved_git_pr_create: false,
        decision_id: format!("dec_{}_1", request.run_id),
    };
    let evaluation = evaluate_action(gadget, &action, &context);
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        match evaluation.decision.decision {
            DecisionKind::Allowed => "action.allowed",
            DecisionKind::Denied => "action.denied",
            DecisionKind::RequiresApproval => "action.requires_approval",
        },
        "gadget",
        &gadget.metadata.name,
        Some(("artifact", "proposed.patch")),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;

    if evaluation.decision.decision != DecisionKind::Allowed {
        return Err(PatchPlanError::PolicyDenied(evaluation.decision.reason));
    }

    let summary = build_summary(&request);
    let proposed_patch = build_proposed_patch(&request);
    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        summary,
    );
    evidence_request.assumptions = build_assumptions(&request);
    evidence_request.extra_artifacts = build_artifacts(&request, &evaluation.decision.reason, proposed_patch);

    let evidence_report = create_observe_bundle(&runs_root, evidence_request)?;
    let evidence_target = evidence_report.bundle_path.to_string_lossy().to_string();
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "evidence.created",
        "gadget",
        &gadget.metadata.name,
        Some(("evidence", evidence_target.as_str())),
        "allowed",
        "Patch Writer created plan-only evidence bundle with proposed patch artifact.",
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.completed",
        "gadget",
        "coordinator",
        None,
        "allowed",
        "Plan-only Patch Writer run completed without modifying files.",
    )?;

    Ok(PatchPlanReport {
        run_id: request.run_id,
        policy_decision: evaluation.decision.decision,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn append_audit(
    ledger_path: &Path,
    request: &PatchPlanRequest,
    state: &mut PlanState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), PatchPlanError> {
    state.next_event_number += 1;
    let event_id = format!("aud_{}_{}", request.run_id, state.next_event_number);
    let event = new_audit_event(
        event_id,
        request.created_at.clone(),
        event_type,
        actor_kind,
        actor_id,
        request.run_id.clone(),
        decision,
        summary,
    );
    let event = match target {
        Some((kind, id)) => with_target(event, kind, id),
        None => event,
    };
    append_event(ledger_path, event)?;
    state.ledger_events_appended += 1;
    Ok(())
}

fn build_summary(request: &PatchPlanRequest) -> String {
    let mut out = String::new();
    out.push_str("Plan-only Patch Writer completed.\n\n");
    out.push_str(&format!("User request: {}\n\n", request.user_prompt));
    if let Some(summary) = request.coordinator_summary.as_deref() {
        out.push_str("Coordinator summary: ");
        out.push_str(summary);
        out.push_str("\n\n");
    }
    out.push_str(&format!("Runtime mode: {}\n", request.runtime_mode.as_str()));
    out.push_str("No files were modified. No commands were executed. The proposed patch is evidence-only and must be reviewed before any future apply step.\n");
    out
}

fn build_assumptions(request: &PatchPlanRequest) -> Vec<String> {
    vec![
        "This was a plan-only Patch Writer run.".to_string(),
        "The Patch Writer did not write files, apply patches, run commands, stage changes, commit, or open a PR.".to_string(),
        format!("Policy decisions were made using {} mode.", request.runtime_mode.as_str()),
        "The proposed patch is a draft artifact for review and may need refinement before approval or application.".to_string(),
    ]
}

fn build_artifacts(
    request: &PatchPlanRequest,
    policy_reason: &str,
    proposed_patch: String,
) -> Vec<EvidenceTextArtifact> {
    let mut artifacts = vec![
        EvidenceTextArtifact::new("proposed_patch", "proposed.patch", proposed_patch),
        EvidenceTextArtifact::new("patch_plan", "patch_plan.md", format_patch_plan(request)),
        EvidenceTextArtifact::new(
            "policy_decision",
            "policy_decision.txt",
            format!("Decision: allowed\nReason: {policy_reason}\n"),
        ),
    ];

    if request.coordinator_summary.is_some() || request.handoff_id.is_some() {
        artifacts.push(EvidenceTextArtifact::new(
            "coordinator_plan",
            "coordinator_plan.md",
            format_coordinator_plan(request),
        ));
    }

    artifacts
}

fn build_proposed_patch(request: &PatchPlanRequest) -> String {
    let escaped_request = request.user_prompt.replace('\n', " ");
    format!(
        "# Plan-only proposed patch\n# This artifact was generated for review only. It was not applied.\n# User request: {escaped_request}\n#\n# A future approved Patch Writer apply step must replace this draft with\n# a concrete, reviewed unified diff before any filesystem write occurs.\ndiff --git a/docs/CHANGE_TARGET.md b/docs/CHANGE_TARGET.md\n--- a/docs/CHANGE_TARGET.md\n+++ b/docs/CHANGE_TARGET.md\n@@ -0,0 +1,5 @@\n+# Proposed change draft\n+\n+Request: {escaped_request}\n+\n+TODO: refine this draft into a concrete scoped patch before approval.\n"
    )
}

fn format_patch_plan(request: &PatchPlanRequest) -> String {
    let mut out = String::new();
    out.push_str("# Patch Writer Plan\n\n");
    out.push_str("This is a plan-only artifact. No filesystem changes were made.\n\n");
    out.push_str("## User Request\n\n");
    out.push_str(&request.user_prompt);
    out.push_str("\n\n## Safety Boundary\n\n");
    out.push_str("- Proposed diff is stored as evidence only.\n");
    out.push_str("- No patch was applied.\n");
    out.push_str("- No tests were run.\n");
    out.push_str("- No Git commands were run.\n");
    out.push_str("- Any later apply step must require approval and path-scoped policy checks.\n");
    out
}

fn format_coordinator_plan(request: &PatchPlanRequest) -> String {
    let mut out = String::new();
    out.push_str("# Coordinator Plan\n\n");
    if let Some(summary) = request.coordinator_summary.as_deref() {
        out.push_str(summary);
        out.push_str("\n\n");
    }
    if let Some(handoff_id) = request.handoff_id.as_deref() {
        out.push_str("Handoff: ");
        out.push_str(handoff_id);
        out.push('\n');
    }
    if let Some(reason) = request.handoff_reason.as_deref() {
        out.push_str("Reason: ");
        out.push_str(reason);
        out.push('\n');
    }
    out
}

fn decision_kind_as_str(decision: &DecisionKind) -> &'static str {
    match decision {
        DecisionKind::Allowed => "allowed",
        DecisionKind::Denied => "denied",
        DecisionKind::RequiresApproval => "requires_approval",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadgets_core::GadgetManifest;

    const PATCH_WRITER: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: patch.writer
  version: 0.1.0
  display_name: Patch Writer Gadget
  description: Proposes scoped repository patches.
runtime:
  model_profile: mock_default
  execution_mode: change
permission_level: change
capabilities:
  - patch.plan
  - file.write
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable: true
    readable_paths:
      - "."
    writable_paths:
      - "src/"
      - "tests/"
      - "docs/"
    denied_paths:
      - ".git/"
tools:
  allowed:
    - patch.plan
    - file.write
evidence:
  required:
    - summary
    - diff
approval:
  required_for:
    - local_write
"#;

    #[test]
    fn proposed_patch_is_marked_plan_only() {
        let request = PatchPlanRequest::plan_patch("run_1", "created", "Add parser tests");
        let patch = build_proposed_patch(&request);
        assert!(patch.contains("Plan-only proposed patch"));
        assert!(patch.contains("It was not applied"));
    }

    #[test]
    fn patch_plan_capability_is_allowed_by_manifest() {
        let gadget = GadgetManifest::from_yaml_str(PATCH_WRITER).unwrap();
        let request = PatchPlanRequest::plan_patch("run_1", "created", "Add parser tests");
        let action = ActionRequest {
            action_request_id: "actreq_1".to_string(),
            run_id: request.run_id.clone(),
            requested_by_gadget: gadget.metadata.name.clone(),
            capability: CapabilityName::new(PATCH_PLAN_CAPABILITY).unwrap(),
            tool: PATCH_PLAN_TOOL.to_string(),
            target: ActionTarget {
                zone: Some(request.zone),
                path: None,
                resource: Some("proposed.patch".to_string()),
            },
            reason: "test".to_string(),
        };
        let context = PolicyContext::safe("dec_1");
        let evaluation = evaluate_action(&gadget, &action, &context);
        assert_eq!(evaluation.decision.decision, DecisionKind::Allowed);
    }
}
