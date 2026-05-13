//! Allowlisted Test Runner provider.
//!
//! This module runs only named test commands that the CLI loaded from the local
//! `.gadgets/config.yaml` allowlist. It does not call model providers, accept
//! commands from model output, apply patches, run Git or PR actions, or perform
//! host/server administration.

use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    create_observe_bundle, default_runs_root, EvidenceError, EvidenceTextArtifact,
    EvidenceWriteRequest,
};
use gadgets_ledger::{append_event, default_ledger_path, new_audit_event, with_target, LedgerError};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_ZONE: &str = "local_repo";
const TEST_RUN_CAPABILITY: &str = "test.run";
const TEST_RUN_TOOL: &str = "test.run";
const DEFAULT_TIMEOUT_SECONDS: u64 = 300;
const MAX_CAPTURE_BYTES: usize = 262_144;

#[derive(Debug)]
pub enum TestRunError {
    Io(std::io::Error),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    InvalidProjectRoot(PathBuf),
    InvalidCommandName(String),
    InvalidCommand(String),
    InvalidWorkingDir(String),
}

impl fmt::Display for TestRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "test runner I/O error: {err}"),
            Self::Ledger(err) => write!(f, "test runner ledger error: {err}"),
            Self::Evidence(err) => write!(f, "test runner evidence error: {err}"),
            Self::Capability(err) => write!(f, "test runner capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "test runner denied by policy: {reason}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::InvalidCommandName(value) => write!(f, "invalid test command name: {value}"),
            Self::InvalidCommand(value) => write!(f, "invalid allowlisted test command: {value}"),
            Self::InvalidWorkingDir(value) => write!(f, "invalid test working_dir: {value}"),
        }
    }
}

impl Error for TestRunError {}

impl From<std::io::Error> for TestRunError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<LedgerError> for TestRunError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for TestRunError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommandSpec {
    pub name: String,
    pub command: String,
    pub working_dir: String,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunRequest {
    pub run_id: String,
    pub created_at: String,
    pub command: TestCommandSpec,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
}

impl TestRunRequest {
    pub fn named_command(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        command: TestCommandSpec,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            command,
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunReport {
    pub run_id: String,
    pub command_name: String,
    pub passed: bool,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct TestRunState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCommand {
    program: String,
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandCapture {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    passed: bool,
    timed_out: bool,
    duration_ms: u128,
}

pub fn run_test_command(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: TestRunRequest,
) -> Result<TestRunReport, TestRunError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(TestRunError::InvalidProjectRoot(project_root.to_path_buf()));
    }

    validate_command_name(&request.command.name)?;
    let parsed = parse_allowlisted_command(&request.command.command)?;
    let project_root = project_root.canonicalize()?;
    let working_dir_rel = normalize_safe_relative_path(&request.command.working_dir)?;
    let working_dir = canonical_working_dir(&project_root, &working_dir_rel)?;
    let working_dir_policy_path = path_for_policy(&working_dir_rel);
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = TestRunState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "test.requested",
        "user",
        "cli",
        Some(("test_command", &request.command.name)),
        "allowed",
        "Named allowlisted test command requested through explicit CLI command.",
    )?;

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(TEST_RUN_CAPABILITY)
            .map_err(|err| TestRunError::Capability(err.to_string()))?,
        tool: TEST_RUN_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(working_dir_policy_path.clone()),
            resource: Some(request.command.name.clone()),
        },
        reason: "Run a named test command loaded from local config allowlist.".to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: false,
        allowlisted_test_command: true,
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
        "policy.checked",
        "gadget",
        &gadget.metadata.name,
        Some(("test_command", &request.command.name)),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
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
        Some(("test_command", &request.command.name)),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
    if evaluation.decision.decision != DecisionKind::Allowed {
        return Err(TestRunError::PolicyDenied(evaluation.decision.reason));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "test.started",
        "gadget",
        &gadget.metadata.name,
        Some(("test_command", &request.command.name)),
        "allowed",
        "Allowlisted test command started without shell, provider tools, patch, Git, PR, or admin actions.",
    )?;

    let timeout_seconds = request
        .command
        .timeout_seconds
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    let capture = execute_command(&parsed, &working_dir, Duration::from_secs(timeout_seconds))?;
    let completed_event = if capture.passed {
        "test.completed"
    } else {
        "test.failed"
    };
    let completed_summary = if capture.timed_out {
        "Allowlisted test command timed out and was recorded as failed."
    } else if capture.passed {
        "Allowlisted test command completed with exit status 0."
    } else {
        "Allowlisted test command completed with nonzero exit status."
    };
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        completed_event,
        "gadget",
        &gadget.metadata.name,
        Some(("test_command", &request.command.name)),
        if capture.passed { "allowed" } else { "failed" },
        completed_summary,
    )?;

    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(&request, &parsed, &working_dir_policy_path, &capture),
    );
    evidence_request.status = if capture.passed {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new("test_command", "test_command.txt", format_command_artifact(&request, &parsed, &working_dir_policy_path)),
        EvidenceTextArtifact::new("stdout", "stdout.txt", capture.stdout.clone()),
        EvidenceTextArtifact::new("stderr", "stderr.txt", capture.stderr.clone()),
        EvidenceTextArtifact::new("exit_status", "exit_status.txt", format_exit_status(&capture)),
        EvidenceTextArtifact::new("duration", "duration.txt", format!("duration_ms={}\n", capture.duration_ms)),
        EvidenceTextArtifact::new("working_dir", "working_dir.txt", format!("{}\n", working_dir_policy_path)),
        EvidenceTextArtifact::new(
            "policy_decision",
            "policy_decision.txt",
            format!(
                "decision={:?}\nreason={}\nmatched_rules={}\n",
                evaluation.decision.decision,
                evaluation.decision.reason,
                evaluation.decision.matched_rules.join(",")
            ),
        ),
    ];

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
        "Test Runner created evidence bundle for allowlisted test command.",
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.completed",
        "gadget",
        "coordinator",
        None,
        if capture.passed { "allowed" } else { "failed" },
        "Allowlisted Test Runner run completed without patch, Git, PR, provider tools, or admin actions.",
    )?;

    Ok(TestRunReport {
        run_id: request.run_id,
        command_name: request.command.name,
        passed: capture.passed,
        timed_out: capture.timed_out,
        exit_code: capture.exit_code,
        duration_ms: capture.duration_ms,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn validate_command_name(value: &str) -> Result<(), TestRunError> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(TestRunError::InvalidCommandName(value.to_string()));
    }
    Ok(())
}

fn parse_allowlisted_command(command: &str) -> Result<ParsedCommand, TestRunError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(TestRunError::InvalidCommand(
            "configured command must not be empty".to_string(),
        ));
    }
    if contains_shell_syntax(trimmed) {
        return Err(TestRunError::InvalidCommand(
            "configured command contains shell metacharacters or unsupported quoting".to_string(),
        ));
    }
    let parts = trimmed
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let Some(program) = parts.first() else {
        return Err(TestRunError::InvalidCommand(
            "configured command must include a program".to_string(),
        ));
    };
    if program.contains('/') || program.contains('\\') {
        return Err(TestRunError::InvalidCommand(
            "configured command program must be a bare executable name".to_string(),
        ));
    }
    Ok(ParsedCommand {
        program: program.clone(),
        args: parts.into_iter().skip(1).collect(),
    })
}

fn contains_shell_syntax(command: &str) -> bool {
    command.contains('\n')
        || command.contains('\r')
        || command.contains(';')
        || command.contains('&')
        || command.contains('|')
        || command.contains('<')
        || command.contains('>')
        || command.contains('`')
        || command.contains('$')
        || command.contains('"')
        || command.contains('\'')
}

fn normalize_safe_relative_path(input: &str) -> Result<PathBuf, TestRunError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(TestRunError::InvalidWorkingDir(
            "working_dir must not be empty".to_string(),
        ));
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(TestRunError::InvalidWorkingDir(format!(
            "{input:?} must be relative"
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(TestRunError::InvalidWorkingDir(format!(
                    "{input:?} must not traverse parents"
                )));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(normalized)
    }
}

fn canonical_working_dir(project_root: &Path, relative: &Path) -> Result<PathBuf, TestRunError> {
    let candidate = project_root.join(relative);
    if !candidate.exists() || !candidate.is_dir() {
        return Err(TestRunError::InvalidWorkingDir(format!(
            "{} must exist and be a directory",
            relative.display()
        )));
    }
    let canonical = fs::canonicalize(&candidate)?;
    if !canonical.starts_with(project_root) {
        return Err(TestRunError::InvalidWorkingDir(format!(
            "{} resolves outside project root",
            relative.display()
        )));
    }
    Ok(canonical)
}

fn path_for_policy(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        ".".to_string()
    } else {
        text
    }
}

fn execute_command(
    parsed: &ParsedCommand,
    working_dir: &Path,
    timeout: Duration,
) -> Result<CommandCapture, TestRunError> {
    let start = Instant::now();
    let mut child = Command::new(&parsed.program)
        .args(&parsed.args)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut timed_out = false;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let output = child.wait_with_output()?;
    let duration_ms = start.elapsed().as_millis();
    let exit_code = output.status.code();
    let passed = output.status.success() && !timed_out;

    Ok(CommandCapture {
        stdout: sanitize_output(&output.stdout),
        stderr: sanitize_output(&output.stderr),
        exit_code,
        passed,
        timed_out,
        duration_ms,
    })
}

fn sanitize_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let redacted = redact_secret_like_lines(&text);
    truncate_output(&redacted)
}

fn redact_secret_like_lines(input: &str) -> String {
    let mut out = String::new();
    for line in input.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("password")
            || lower.contains("secret")
            || lower.contains("token")
            || lower.contains("api_key")
            || lower.contains("apikey")
            || lower.contains("credential")
        {
            out.push_str("[redacted secret-like output line]\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn truncate_output(input: &str) -> String {
    if input.len() <= MAX_CAPTURE_BYTES {
        return input.to_string();
    }
    let mut end = MAX_CAPTURE_BYTES;
    while !input.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = input[..end].to_string();
    out.push_str("\n[output truncated by Gadgets Test Runner]\n");
    out
}

fn append_audit(
    ledger_path: &Path,
    request: &TestRunRequest,
    state: &mut TestRunState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), TestRunError> {
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

fn build_summary(
    request: &TestRunRequest,
    parsed: &ParsedCommand,
    working_dir: &str,
    capture: &CommandCapture,
) -> String {
    let status = if capture.passed { "passed" } else { "failed" };
    let mut out = String::new();
    out.push_str("Allowlisted test command completed.\n\n");
    out.push_str(&format!("Command name: {}\n", request.command.name));
    out.push_str(&format!("Program: {}\n", parsed.program));
    out.push_str(&format!("Args: {}\n", parsed.args.join(" ")));
    out.push_str(&format!("Working dir: {}\n", working_dir));
    out.push_str(&format!("Status: {}\n", status));
    out.push_str(&format!("Timed out: {}\n", capture.timed_out));
    out.push_str(&format!("Exit code: {}\n", display_exit_code(capture.exit_code)));
    out.push_str(&format!("Duration ms: {}\n\n", capture.duration_ms));
    out.push_str("No patch, Git, PR, provider tool, Linux admin, database, cloud, or deployment action was executed by this provider.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The command string came from .gadgets/config.yaml, not model output.".to_string(),
        "The command was launched directly without sh -c or provider-side tools.".to_string(),
        "The configured working_dir was validated as a project-relative directory.".to_string(),
        "Captured stdout and stderr were capped and secret-like lines were redacted before evidence write.".to_string(),
    ]
}

fn format_command_artifact(
    request: &TestRunRequest,
    parsed: &ParsedCommand,
    working_dir: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("name={}\n", request.command.name));
    out.push_str(&format!("command={}\n", request.command.command));
    out.push_str(&format!("program={}\n", parsed.program));
    out.push_str(&format!("args={}\n", parsed.args.join(" ")));
    out.push_str(&format!("working_dir={}\n", working_dir));
    out.push_str(&format!(
        "timeout_seconds={}\n",
        request.command.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS)
    ));
    out
}

fn format_exit_status(capture: &CommandCapture) -> String {
    let mut out = String::new();
    out.push_str(&format!("passed={}\n", capture.passed));
    out.push_str(&format!("timed_out={}\n", capture.timed_out));
    out.push_str(&format!("exit_code={}\n", display_exit_code(capture.exit_code)));
    out
}

fn display_exit_code(value: Option<i32>) -> String {
    value
        .map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_string())
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

    #[test]
    fn parses_simple_allowlisted_command() {
        let parsed = parse_allowlisted_command("cargo test --all").unwrap();
        assert_eq!(parsed.program, "cargo");
        assert_eq!(parsed.args, vec!["test", "--all"]);
    }

    #[test]
    fn rejects_shell_metacharacters() {
        assert!(parse_allowlisted_command("cargo test && rm -rf target").is_err());
        assert!(parse_allowlisted_command("cargo test | cat").is_err());
        assert!(parse_allowlisted_command("cargo test > out.txt").is_err());
    }

    #[test]
    fn rejects_parent_traversal_working_dir() {
        assert!(normalize_safe_relative_path("../outside").is_err());
    }

    #[test]
    fn redacts_secret_like_output_lines() {
        let output = sanitize_output(b"ok\npassword=abc\nsecret token\ndone\n");
        assert!(output.contains("ok"));
        assert!(output.contains("done"));
        assert!(!output.contains("abc"));
        assert!(!output.contains("secret token"));
    }
}
