//! Built-in deterministic policy checks.
//!
//! This crate implements the first local policy engine for Gadgets Framework.
//! It does not execute tools, call providers, write evidence, or append audit
//! records. It only evaluates structured action requests against manifest
//! capabilities, zones, filesystem boundaries, active mode, and approval state.

use gadgets_core::{
    ActionRequest, BoundarySet, CapabilityName, DecisionKind, FilesystemBoundary, GadgetManifest,
    PermissionLevel, PolicyDecision,
};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Safe,
    Team,
    Production,
}

impl RuntimeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Team => "team",
            Self::Production => "production",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyContext {
    pub mode: RuntimeMode,
    pub approval_present: bool,
    pub allowlisted_test_command: bool,
    pub allowlisted_git_branch_create: bool,
    pub approved_git_commit: bool,
    pub approved_git_pr_create: bool,
    pub decision_id: String,
}

impl PolicyContext {
    pub fn safe(decision_id: impl Into<String>) -> Self {
        Self {
            mode: RuntimeMode::Safe,
            approval_present: false,
            allowlisted_test_command: false,
            allowlisted_git_branch_create: false,
            approved_git_commit: false,
            approved_git_pr_create: false,
            decision_id: decision_id.into(),
        }
    }

    pub fn with_approval(mut self) -> Self {
        self.approval_present = true;
        self
    }

    pub fn with_allowlisted_test_command(mut self) -> Self {
        self.allowlisted_test_command = true;
        self
    }

    pub fn with_allowlisted_git_branch_create(mut self) -> Self {
        self.allowlisted_git_branch_create = true;
        self
    }

    pub fn with_approved_git_commit(mut self) -> Self {
        self.approved_git_commit = true;
        self.approval_present = true;
        self
    }

    pub fn with_approved_git_pr_create(mut self) -> Self {
        self.approved_git_pr_create = true;
        self.approval_present = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluation {
    pub decision: PolicyDecision,
    pub findings: Vec<PolicyFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFinding {
    pub rule: String,
    pub message: String,
}

pub fn evaluate_action(
    gadget: &GadgetManifest,
    request: &ActionRequest,
    context: &PolicyContext,
) -> PolicyEvaluation {
    let mut findings = Vec::new();
    let mut matched_rules = Vec::new();

    if request.requested_by_gadget != gadget.metadata.name {
        findings.push(finding(
            "gadget_identity_mismatch",
            format!(
                "action requested by {}, but manifest is for {}",
                request.requested_by_gadget, gadget.metadata.name
            ),
        ));
    } else {
        matched_rules.push("gadget_identity_match".to_string());
    }

    if !gadget_has_capability(gadget, &request.capability) {
        findings.push(finding(
            "capability_not_declared",
            format!(
                "Gadget {} does not declare capability {}",
                gadget.metadata.name, request.capability
            ),
        ));
    } else {
        matched_rules.push("capability_declared".to_string());
    }

    let capability_level = request.capability.estimated_level();
    let allowlisted_test_run = request.capability.as_str() == "test.run"
        && request.tool == "test.run"
        && context.allowlisted_test_command;
    let allowlisted_git_branch_create = request.capability.as_str() == "git.branch.create"
        && request.tool == "git.branch.create"
        && context.allowlisted_git_branch_create;
    let approved_git_commit = request.capability.as_str() == "git.commit.create"
        && request.tool == "git.commit.create"
        && context.approved_git_commit;
    let approved_git_pr_create = request.capability.as_str() == "git.pr.create"
        && request.tool == "git.pr.create"
        && context.approved_git_pr_create;
    let boundary_level = if allowlisted_test_run || allowlisted_git_branch_create || approved_git_commit || approved_git_pr_create {
        PermissionLevel::Observe
    } else {
        capability_level
    };

    if capability_level > gadget.permission_level {
        findings.push(finding(
            "permission_level_exceeded",
            format!(
                "capability {} requires {}, but Gadget permission level is {}",
                request.capability, capability_level, gadget.permission_level
            ),
        ));
    } else {
        matched_rules.push("permission_level_allows_capability".to_string());
    }

    if !tool_allowed(gadget, &request.tool) {
        findings.push(finding(
            "tool_not_allowed",
            format!(
                "tool {} is not allowlisted for Gadget {}",
                request.tool, gadget.metadata.name
            ),
        ));
    } else {
        matched_rules.push("tool_allowed".to_string());
    }

    match request.target.zone.as_deref() {
        Some(zone) if gadget.boundaries.zones.iter().any(|allowed| allowed == zone) => {
            matched_rules.push("zone_allowed".to_string());
        }
        Some(zone) => findings.push(finding(
            "zone_not_allowed",
            format!("zone {zone} is not allowed for Gadget {}", gadget.metadata.name),
        )),
        None => findings.push(finding(
            "target_zone_missing",
            "action target must include a zone".to_string(),
        )),
    }

    if let Some(path) = request.target.path.as_deref() {
        evaluate_filesystem_path(
            &gadget.boundaries,
            path,
            boundary_level,
            &mut findings,
            &mut matched_rules,
        );
    } else if is_filesystem_action(&request.capability, &request.tool) {
        findings.push(finding(
            "target_path_missing",
            "filesystem action target must include a path".to_string(),
        ));
    }

    match context.mode {
        RuntimeMode::Safe => {
            matched_rules.push("mode_safe".to_string());
            if capability_level == PermissionLevel::Release {
                findings.push(finding(
                    "safe_mode_blocks_release",
                    format!("Safe Mode blocks release-level capability {}", request.capability),
                ));
            }
        }
        RuntimeMode::Team => {
            matched_rules.push("mode_team".to_string());
            if capability_level == PermissionLevel::Release {
                findings.push(finding(
                    "team_mode_requires_production_gate",
                    format!("Team Mode blocks release-level capability {} without production gate", request.capability),
                ));
            }
        }
        RuntimeMode::Production => {
            matched_rules.push("mode_production".to_string());
        }
    }

    if !findings.is_empty() {
        let reason = findings
            .iter()
            .map(|item| format!("{}: {}", item.rule, item.message))
            .collect::<Vec<_>>()
            .join("; ");
        return evaluation(
            context,
            request,
            DecisionKind::Denied,
            reason,
            false,
            matched_rules,
            findings,
        );
    }

    if capability_level >= PermissionLevel::Change {
        matched_rules.push("mutating_action_detected".to_string());
        if allowlisted_test_run {
            matched_rules.push("allowlisted_test_command".to_string());
            return evaluation(
                context,
                request,
                DecisionKind::Allowed,
                "test.run allowed because the command name was loaded from the local config allowlist".to_string(),
                false,
                matched_rules,
                findings,
            );
        }
        if allowlisted_git_branch_create {
            matched_rules.push("allowlisted_git_branch_create".to_string());
            return evaluation(
                context,
                request,
                DecisionKind::Allowed,
                "git.branch.create allowed because the branch name was validated by the runtime and protected branches were rejected".to_string(),
                false,
                matched_rules,
                findings,
            );
        }
        if approved_git_commit && context.approval_present {
            matched_rules.push("approved_git_commit".to_string());
            return evaluation(
                context,
                request,
                DecisionKind::Allowed,
                "git.commit.create allowed because the runtime verified a scoped approval and will stage only approved patch files".to_string(),
                false,
                matched_rules,
                findings,
            );
        }
        if approved_git_pr_create && context.approval_present {
            matched_rules.push("approved_git_pr_create".to_string());
            return evaluation(
                context,
                request,
                DecisionKind::Allowed,
                "git.pr.create allowed because the runtime verified scoped approval, local PR body evidence, and explicit remote PR config".to_string(),
                false,
                matched_rules,
                findings,
            );
        }
        if !context.approval_present {
            return evaluation(
                context,
                request,
                DecisionKind::RequiresApproval,
                "mutating action requires approval before execution".to_string(),
                true,
                matched_rules,
                findings,
            );
        }
        matched_rules.push("approval_present".to_string());
    }

    evaluation(
        context,
        request,
        DecisionKind::Allowed,
        "action allowed by built-in policy".to_string(),
        false,
        matched_rules,
        findings,
    )
}

fn evaluation(
    context: &PolicyContext,
    request: &ActionRequest,
    decision: DecisionKind,
    reason: String,
    requires_approval: bool,
    matched_rules: Vec<String>,
    findings: Vec<PolicyFinding>,
) -> PolicyEvaluation {
    PolicyEvaluation {
        decision: PolicyDecision {
            decision_id: context.decision_id.clone(),
            action_request_id: request.action_request_id.clone(),
            decision,
            reason,
            requires_approval,
            matched_rules,
        },
        findings,
    }
}

fn gadget_has_capability(gadget: &GadgetManifest, capability: &CapabilityName) -> bool {
    gadget.capabilities.iter().any(|item| item == capability)
}

fn tool_allowed(gadget: &GadgetManifest, tool: &str) -> bool {
    gadget.tools.allowed.iter().any(|item| item == tool)
}

fn is_filesystem_action(capability: &CapabilityName, tool: &str) -> bool {
    capability.as_str().starts_with("file.")
        || capability.as_str().starts_with("repo.")
        || tool.starts_with("file.")
        || tool.starts_with("repo.")
}

fn evaluate_filesystem_path(
    boundaries: &BoundarySet,
    path: &str,
    capability_level: PermissionLevel,
    findings: &mut Vec<PolicyFinding>,
    matched_rules: &mut Vec<String>,
) {
    let Some(filesystem) = boundaries.filesystem.as_ref() else {
        findings.push(finding(
            "filesystem_boundary_missing",
            "filesystem target requires filesystem boundaries".to_string(),
        ));
        return;
    };

    let Some(normalized) = normalize_relative_path(path) else {
        findings.push(finding(
            "path_not_relative_safe",
            format!("path {path:?} must be relative and must not traverse parents"),
        ));
        return;
    };

    if matches_denied_path(filesystem, &normalized) {
        findings.push(finding(
            "path_denied",
            format!("path {} matches denied filesystem boundary", normalized.display()),
        ));
        return;
    }
    matched_rules.push("path_not_denied".to_string());

    if !is_inside_any_root(filesystem, &normalized) {
        findings.push(finding(
            "path_outside_roots",
            format!("path {} is outside configured filesystem roots", normalized.display()),
        ));
        return;
    }
    matched_rules.push("path_inside_roots".to_string());

    if capability_level >= PermissionLevel::Change {
        if !filesystem.writable && filesystem.writable_paths.is_empty() {
            findings.push(finding(
                "filesystem_not_writable",
                "filesystem boundary does not allow writes".to_string(),
            ));
            return;
        }

        if !filesystem.writable_paths.is_empty()
            && !is_inside_any_path(&filesystem.writable_paths, &normalized)
        {
            findings.push(finding(
                "path_not_writable",
                format!("path {} is not inside writable paths", normalized.display()),
            ));
            return;
        }
        matched_rules.push("path_writable".to_string());
    } else if !filesystem.readable_paths.is_empty()
        && !is_inside_any_path(&filesystem.readable_paths, &normalized)
    {
        findings.push(finding(
            "path_not_readable",
            format!("path {} is not inside readable paths", normalized.display()),
        ));
    } else {
        matched_rules.push("path_readable".to_string());
    }
}

fn normalize_relative_path(input: &str) -> Option<PathBuf> {
    let path = Path::new(input);
    if path.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if normalized.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(normalized)
    }
}

fn is_inside_any_root(filesystem: &FilesystemBoundary, path: &Path) -> bool {
    if filesystem.roots.is_empty() {
        return true;
    }
    is_inside_any_path(&filesystem.roots, path)
}

fn is_inside_any_path(allowed: &[String], path: &Path) -> bool {
    allowed.iter().any(|item| {
        normalize_relative_path(item)
            .map(|allowed_path| allowed_path == Path::new(".") || path == allowed_path || path.starts_with(&allowed_path))
            .unwrap_or(false)
    })
}

fn matches_denied_path(filesystem: &FilesystemBoundary, path: &Path) -> bool {
    filesystem
        .denied_paths
        .iter()
        .any(|pattern| matches_boundary_pattern(pattern, path))
}

fn matches_boundary_pattern(pattern: &str, path: &Path) -> bool {
    let path_text = path.to_string_lossy().replace('\\', "/");
    let path_lower = path_text.to_ascii_lowercase();
    let pattern_lower = pattern.to_ascii_lowercase();

    if pattern_lower == path_lower {
        return true;
    }

    if let Some(prefix) = pattern_lower.strip_suffix('/') {
        return path_lower == prefix || path_lower.starts_with(&format!("{prefix}/"));
    }

    if let Some(suffix) = pattern_lower.strip_prefix("**/*") {
        return path_lower.contains(suffix.trim_matches('*'));
    }

    if let Some(suffix) = pattern_lower.strip_prefix("**/") {
        return path_lower.ends_with(suffix);
    }

    if pattern_lower.starts_with("**/*.") {
        let suffix = pattern_lower.trim_start_matches("**/*");
        return path_lower.ends_with(suffix);
    }

    pattern_lower.contains('*') && simple_wildcard_match(&pattern_lower, &path_lower)
}

fn simple_wildcard_match(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return true;
    }

    let mut remainder = value;
    for part in parts {
        let Some(idx) = remainder.find(part) else {
            return false;
        };
        remainder = &remainder[idx + part.len()..];
    }
    true
}

fn finding(rule: impl Into<String>, message: impl Into<String>) -> PolicyFinding {
    PolicyFinding {
        rule: rule.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadgets_core::{ActionTarget, GadgetManifest};

    const READ_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: filesystem.read
  version: 0.1.0
  display_name: Filesystem Read Gadget
  description: Reads scoped project files.
runtime:
  model_profile: mock_default
  execution_mode: observe
permission_level: observe
capabilities:
  - file.read
  - file.search
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    readable_paths:
      - "."
    writable: false
    denied_paths:
      - ".git/"
      - ".env"
      - "secrets/"
      - "**/*.pem"
      - "**/*.key"
      - "**/*secret*"
      - "**/*credential*"
tools:
  allowed:
    - file.read
    - file.search
evidence:
  required:
    - summary
approval:
  required_for: []
"#;

    const PATCH_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: patch.writer
  version: 0.1.0
  display_name: Patch Writer Gadget
  description: Writes scoped project patches.
runtime:
  model_profile: mock_default
  execution_mode: change
permission_level: change
capabilities:
  - file.write
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable_paths:
      - "src/"
      - "tests/"
    writable: false
    denied_paths:
      - ".git/"
      - ".env"
tools:
  allowed:
    - file.write
evidence:
  required:
    - diff
approval:
  required_for:
    - local_write
"#;

    const RELEASE_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: firewall.executor
  version: 0.1.0
  display_name: Firewall Executor Gadget
  description: Applies approved firewall changes.
runtime:
  model_profile: mock_default
  execution_mode: release
permission_level: release
capabilities:
  - linux.firewall.apply
boundaries:
  zones:
    - local_host_change
tools:
  allowed:
    - linux.firewall.apply
evidence:
  required:
    - before_state
approval:
  required_for:
    - firewall_change
"#;

    fn action(capability: &str, tool: &str, gadget: &str, zone: &str, path: Option<&str>) -> ActionRequest {
        ActionRequest {
            action_request_id: "actreq_1".to_string(),
            run_id: "run_1".to_string(),
            requested_by_gadget: gadget.to_string(),
            capability: CapabilityName::new(capability).unwrap(),
            tool: tool.to_string(),
            target: ActionTarget {
                zone: Some(zone.to_string()),
                path: path.map(ToString::to_string),
                resource: None,
            },
            reason: "test".to_string(),
        }
    }

    #[test]
    fn allows_declared_read_inside_zone_and_path() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = action("file.read", "file.read", "filesystem.read", "local_repo", Some("README.md"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::Allowed);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn denies_path_traversal() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = action("file.read", "file.read", "filesystem.read", "local_repo", Some("../secret.txt"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("path_not_relative_safe"));
    }

    #[test]
    fn denies_configured_secret_paths() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        for path in [".env", ".git/config", "secrets/api.txt", "certs/private.key", "notes/secret-token.txt"] {
            let request = action("file.read", "file.read", "filesystem.read", "local_repo", Some(path));
            let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
            assert_eq!(result.decision.decision, DecisionKind::Denied, "{path}");
            assert!(result.decision.reason.contains("path_denied"), "{path}");
        }
    }

    #[test]
    fn denies_missing_capability() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = action("file.write", "file.read", "filesystem.read", "local_repo", Some("README.md"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("capability_not_declared"));
    }

    #[test]
    fn denies_tool_not_allowlisted() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = action("file.read", "shell.exec", "filesystem.read", "local_repo", Some("README.md"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("tool_not_allowed"));
    }

    #[test]
    fn denies_zone_not_allowed() {
        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = action("file.read", "file.read", "filesystem.read", "prod", Some("README.md"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("zone_not_allowed"));
    }

    #[test]
    fn write_requires_approval() {
        let gadget = GadgetManifest::from_yaml_str(PATCH_GADGET).unwrap();
        let request = action("file.write", "file.write", "patch.writer", "local_repo", Some("src/lib.rs"));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::RequiresApproval);
        assert!(result.decision.requires_approval);
    }

    #[test]
    fn approved_write_inside_writable_path_is_allowed() {
        let gadget = GadgetManifest::from_yaml_str(PATCH_GADGET).unwrap();
        let request = action("file.write", "file.write", "patch.writer", "local_repo", Some("tests/policy.rs"));
        let context = PolicyContext::safe("dec_1").with_approval();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Allowed);
    }

    #[test]
    fn approved_write_outside_writable_path_is_denied() {
        let gadget = GadgetManifest::from_yaml_str(PATCH_GADGET).unwrap();
        let request = action("file.write", "file.write", "patch.writer", "local_repo", Some("README.md"));
        let context = PolicyContext::safe("dec_1").with_approval();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("path_not_writable"));
    }

    #[test]
    fn safe_mode_blocks_release_even_with_approval() {
        let gadget = GadgetManifest::from_yaml_str(RELEASE_GADGET).unwrap();
        let request = action(
            "linux.firewall.apply",
            "linux.firewall.apply",
            "firewall.executor",
            "local_host_change",
            None,
        );
        let context = PolicyContext::safe("dec_1").with_approval();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("safe_mode_blocks_release"));
    }


    const TEST_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: test.runner
  version: 0.1.0
  display_name: Test Runner Gadget
  description: Runs configured tests.
runtime:
  model_profile: mock_default
  execution_mode: change
permission_level: change
capabilities:
  - test.run
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable: false
    denied_paths:
      - ".git/"
      - ".gadgets/"
      - ".env"
tools:
  allowed:
    - test.run
evidence:
  required:
    - summary
approval:
  required_for:
    - test_run
"#;

    #[test]
    fn allowlisted_test_run_is_allowed_without_patch_approval() {
        let gadget = GadgetManifest::from_yaml_str(TEST_GADGET).unwrap();
        let request = action("test.run", "test.run", "test.runner", "local_repo", Some("."));
        let context = PolicyContext::safe("dec_1").with_allowlisted_test_command();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Allowed);
        assert!(result
            .decision
            .matched_rules
            .iter()
            .any(|rule| rule == "allowlisted_test_command"));
    }

    #[test]
    fn test_run_requires_allowlist_context() {
        let gadget = GadgetManifest::from_yaml_str(TEST_GADGET).unwrap();
        let request = action("test.run", "test.run", "test.runner", "local_repo", Some("."));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::RequiresApproval);
    }

    #[test]
    fn allowlisted_test_run_still_checks_denied_paths() {
        let gadget = GadgetManifest::from_yaml_str(TEST_GADGET).unwrap();
        let request = action("test.run", "test.run", "test.runner", "local_repo", Some(".gadgets"));
        let context = PolicyContext::safe("dec_1").with_allowlisted_test_command();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Denied);
        assert!(result.decision.reason.contains("path_denied"));
    }


    const GIT_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: git.pr
  version: 0.1.0
  display_name: Git Pull Request Gadget
  description: Reads local Git status and creates protected local branches and approved commits.
runtime:
  model_profile: mock_default
  execution_mode: change
permission_level: change
capabilities:
  - git.status
  - git.branch.create
  - git.commit.create
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable: false
    denied_paths:
      - ".git/"
      - ".gadgets/"
tools:
  allowed:
    - git.status
    - git.branch.create
    - git.commit.create
evidence:
  required:
    - summary
approval:
  required_for:
    - git_write
"#;

    #[test]
    fn allowlisted_git_branch_create_is_allowed_without_patch_approval() {
        let gadget = GadgetManifest::from_yaml_str(GIT_GADGET).unwrap();
        let request = action("git.branch.create", "git.branch.create", "git.pr", "local_repo", Some("."));
        let context = PolicyContext::safe("dec_1").with_allowlisted_git_branch_create();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Allowed);
        assert!(result
            .decision
            .matched_rules
            .iter()
            .any(|rule| rule == "allowlisted_git_branch_create"));
    }

    #[test]
    fn git_branch_create_requires_allowlist_context() {
        let gadget = GadgetManifest::from_yaml_str(GIT_GADGET).unwrap();
        let request = action("git.branch.create", "git.branch.create", "git.pr", "local_repo", Some("."));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::RequiresApproval);
    }


    #[test]
    fn approved_git_commit_is_allowed_with_verified_approval_context() {
        let gadget = GadgetManifest::from_yaml_str(GIT_GADGET).unwrap();
        let request = action("git.commit.create", "git.commit.create", "git.pr", "local_repo", Some("."));
        let context = PolicyContext::safe("dec_1").with_approved_git_commit();
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::Allowed);
        assert!(result
            .decision
            .matched_rules
            .iter()
            .any(|rule| rule == "approved_git_commit"));
    }

    #[test]
    fn git_commit_requires_approval_context() {
        let gadget = GadgetManifest::from_yaml_str(GIT_GADGET).unwrap();
        let request = action("git.commit.create", "git.commit.create", "git.pr", "local_repo", Some("."));
        let result = evaluate_action(&gadget, &request, &PolicyContext::safe("dec_1"));
        assert_eq!(result.decision.decision, DecisionKind::RequiresApproval);
    }

    #[test]
    fn production_mode_release_requires_approval() {
        let gadget = GadgetManifest::from_yaml_str(RELEASE_GADGET).unwrap();
        let request = action(
            "linux.firewall.apply",
            "linux.firewall.apply",
            "firewall.executor",
            "local_host_change",
            None,
        );
        let context = PolicyContext {
            mode: RuntimeMode::Production,
            approval_present: false,
            allowlisted_test_command: false,
            allowlisted_git_branch_create: false,
            approved_git_commit: false,
            approved_git_pr_create: false,
            decision_id: "dec_1".to_string(),
        };
        let result = evaluate_action(&gadget, &request, &context);
        assert_eq!(result.decision.decision, DecisionKind::RequiresApproval);
    }
}
