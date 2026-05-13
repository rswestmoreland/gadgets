use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const GADGETS_DIR: &str = ".gadgets";

const STATE_DIRS: &[&str] = &[
    "packs",
    "gadgets",
    "zones",
    "runs",
    "ledger",
    "evidence",
    "approvals",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitReport {
    pub project_root: PathBuf,
    pub gadgets_dir: PathBuf,
    pub created_dirs: Vec<PathBuf>,
    pub existing_dirs: Vec<PathBuf>,
    pub created_files: Vec<PathBuf>,
    pub existing_files: Vec<PathBuf>,
}

impl InitReport {
    fn new(project_root: PathBuf, gadgets_dir: PathBuf) -> Self {
        Self {
            project_root,
            gadgets_dir,
            created_dirs: Vec::new(),
            existing_dirs: Vec::new(),
            created_files: Vec::new(),
            existing_files: Vec::new(),
        }
    }

    pub fn created_anything(&self) -> bool {
        !self.created_dirs.is_empty() || !self.created_files.is_empty()
    }
}

pub fn init_project(project_root: &Path) -> io::Result<InitReport> {
    let project_root = project_root.to_path_buf();
    let gadgets_dir = project_root.join(GADGETS_DIR);
    let mut report = InitReport::new(project_root, gadgets_dir.clone());

    create_dir_recorded(&gadgets_dir, &mut report)?;
    for rel in STATE_DIRS {
        create_dir_recorded(&gadgets_dir.join(rel), &mut report)?;
    }

    create_file_if_missing(
        &gadgets_dir.join("config.yaml"),
        default_config_yaml().as_bytes(),
        &mut report,
    )?;
    create_file_if_missing(
        &gadgets_dir.join("README.md"),
        gadgets_readme().as_bytes(),
        &mut report,
    )?;
    create_file_if_missing(
        &gadgets_dir.join(".gitignore"),
        gadgets_gitignore().as_bytes(),
        &mut report,
    )?;
    create_file_if_missing(&gadgets_dir.join("ledger/events.jsonl"), b"", &mut report)?;

    Ok(report)
}

fn create_dir_recorded(path: &Path, report: &mut InitReport) -> io::Result<()> {
    if path.exists() {
        report.existing_dirs.push(path.to_path_buf());
        return Ok(());
    }

    fs::create_dir_all(path)?;
    report.created_dirs.push(path.to_path_buf());
    Ok(())
}

fn create_file_if_missing(path: &Path, content: &[u8], report: &mut InitReport) -> io::Result<()> {
    if path.exists() {
        report.existing_files.push(path.to_path_buf());
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    report.created_files.push(path.to_path_buf());
    Ok(())
}

pub fn default_config_yaml() -> &'static str {
    r#"schema_version: gadgets.framework/config/v0.1

mode: safe
default_model_profile: mock_default

model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock

  # OpenAI is supported but disabled by default. To use it, set
  # default_model_profile: openai_default and export OPENAI_API_KEY.
  # openai_default:
  #   provider: openai
  #   model: gpt-5.5
  #   api_key_env: OPENAI_API_KEY

  # Anthropic is supported but disabled by default. To use it, set
  # default_model_profile: anthropic_default and export ANTHROPIC_API_KEY.
  # anthropic_default:
  #   provider: anthropic
  #   model: claude-sonnet-4-6
  #   api_key_env: ANTHROPIC_API_KEY

installed_packs:
  - developer

zones:
  local_repo:
    type: local_repo
    root: "."
    readable_paths:
      - "."
    writable_paths: []
    denied_paths:
      - ".git/"
      - ".gadgets/"
      - ".env"
      - "secrets/"
      - "**/*.pem"
      - "**/*.key"
      - "**/*secret*"
      - "**/*credential*"

audit:
  ledger_path: ".gadgets/ledger/events.jsonl"

evidence:
  root: ".gadgets/runs"

approval:
  require_for_all_writes: true
  # Optional patch approval expiration uses strict UTC RFC3339 without fractional seconds.
  # Example: gadgets approval request-patch <run-id> --expires-at 2999-01-01T00:00:00Z

# Test commands are disabled by default. Add named entries only after review.
# The Test Runner uses the name only; it does not accept raw command strings
# from prompts or model output. Commands are launched without sh -c.
#
# test_commands:
#   - name: cargo_test
#     command: cargo test
#     working_dir: "."
#     timeout_seconds: 300
test_commands: []

git:
  # Remote PR creation is disabled by default. When enabled, Gadgets creates
  # one GitHub pull request from verified approval and local PR-body evidence.
  # It does not push branches; the head branch must already exist remotely.
  remote_pr:
    enabled: false
    provider: github
    owner: ""
    repo: ""
    api_base: https://api.github.com
    token_env: GITHUB_TOKEN
    default_base_branch: main

  protected_branches:
    - main
    - master
    - trunk
    - production
    - prod
    - release/
"#
}

pub fn gadgets_readme() -> &'static str {
    r#"# Local Gadgets State

This directory stores local Gadgets Framework project state.

Generated defaults:

- Safe Mode is enabled.
- The mock provider is configured by default.
- OpenAI can be enabled by editing `config.yaml` and setting `OPENAI_API_KEY`.
- The Developer Pack is selected.
- File writes require approval.
- Test commands are disabled until explicitly allowlisted.
- Protected Git branches are configured by default.
- Secret-like paths are denied by default.

Volatile run data is ignored by `.gadgets/.gitignore`:

- runs/
- ledger/
- evidence/
- approvals/

Review `config.yaml` before enabling additional packs, providers, test commands, or Git branch behavior.
"#
}

pub fn gadgets_gitignore() -> &'static str {
    r#"runs/
ledger/
evidence/
approvals/
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_config_uses_safe_mode_and_mock_provider() {
        let config = default_config_yaml();
        assert!(config.contains("mode: safe"));
        assert!(config.contains("provider: mock"));
        assert!(config.contains("provider: openai"));
        assert!(config.contains("provider: anthropic"));
        assert!(config.contains("OPENAI_API_KEY"));
        assert!(config.contains("default_model_profile: mock_default"));
        assert!(config.contains("require_for_all_writes: true"));
        assert!(config.contains("test_commands: []"));
        assert!(config.contains("cargo_test"));
        assert!(config.contains("remote_pr:"));
        assert!(config.contains("enabled: false"));
        assert!(config.contains("token_env: GITHUB_TOKEN"));
        assert!(config.contains("protected_branches:"));
        assert!(config.contains("release/"));
        assert!(config.contains(".git/"));
        assert!(config.contains(".gadgets/"));
        assert!(config.contains(".env"));
    }

    #[test]
    fn init_project_is_idempotent() {
        let temp = std::env::temp_dir().join(format!(
            "gadgets-init-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp).unwrap();

        let first = init_project(&temp).unwrap();
        assert!(first.created_anything());
        assert!(temp.join(".gadgets/config.yaml").exists());
        assert!(temp.join(".gadgets/packs").is_dir());
        assert!(temp.join(".gadgets/ledger").is_dir());
        assert!(temp.join(".gadgets/ledger/events.jsonl").exists());

        let second = init_project(&temp).unwrap();
        assert!(!second.created_anything());
        assert!(second
            .existing_files
            .iter()
            .any(|p| p.ends_with("config.yaml")));

        fs::remove_dir_all(&temp).unwrap();
    }
}
