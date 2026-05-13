//! Local runtime configuration loading for the Gadgets CLI.
//!
//! Step 9 keeps provider selection outside the hardcoded Coordinator flow. The
//! CLI now loads `.gadgets/config.yaml`, selects a model profile, and then
//! instantiates the configured provider adapter. The mock provider remains the
//! default; OpenAI and Anthropic are available behind the same provider trait.

use gadgets_policy::RuntimeMode;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_SCHEMA_VERSION: &str = "gadgets.framework/config/v0.1";
pub const DEFAULT_MODEL_PROFILE: &str = "mock_default";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub schema_version: String,
    pub mode: String,
    #[serde(default)]
    pub default_model_profile: Option<String>,
    pub model_profiles: BTreeMap<String, ModelProfileConfig>,
    #[serde(default)]
    pub installed_packs: Vec<String>,
    #[serde(default)]
    pub test_commands: Vec<TestCommandConfig>,
    #[serde(default)]
    pub git: GitConfig,
}

impl RuntimeConfig {
    pub fn from_yaml_str(input: &str) -> Result<Self, ConfigError> {
        let value: Self = serde_yaml::from_str(input).map_err(ConfigError::Yaml)?;
        value.validate()?;
        Ok(value)
    }

    pub fn runtime_mode(&self) -> Result<RuntimeMode, ConfigError> {
        parse_runtime_mode(&self.mode)
    }

    pub fn selected_model_profile_name(&self) -> Result<&str, ConfigError> {
        if let Some(name) = self.default_model_profile.as_deref() {
            if self.model_profiles.contains_key(name) {
                return Ok(name);
            }
            return Err(ConfigError::MissingModelProfile(name.to_string()));
        }

        if self.model_profiles.contains_key(DEFAULT_MODEL_PROFILE) {
            return Ok(DEFAULT_MODEL_PROFILE);
        }

        self.model_profiles
            .keys()
            .next()
            .map(String::as_str)
            .ok_or(ConfigError::NoModelProfiles)
    }

    pub fn selected_model_profile(&self) -> Result<SelectedModelProfile<'_>, ConfigError> {
        let name = self.selected_model_profile_name()?;
        let profile = self
            .model_profiles
            .get(name)
            .ok_or_else(|| ConfigError::MissingModelProfile(name.to_string()))?;
        Ok(SelectedModelProfile { name, profile })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchema(self.schema_version.clone()));
        }

        self.runtime_mode()?;

        if self.model_profiles.is_empty() {
            return Err(ConfigError::NoModelProfiles);
        }

        for (name, profile) in &self.model_profiles {
            if !valid_profile_name(name) {
                return Err(ConfigError::InvalidModelProfileName(name.clone()));
            }
            if profile.provider.trim().is_empty() {
                return Err(ConfigError::InvalidModelProfile(format!(
                    "model profile {name} has an empty provider"
                )));
            }
            if profile.model.trim().is_empty() {
                return Err(ConfigError::InvalidModelProfile(format!(
                    "model profile {name} has an empty model"
                )));
            }
        }

        self.selected_model_profile_name()?;

        validate_test_commands(&self.test_commands)?;
        validate_git_config(&self.git)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TestCommandConfig {
    pub name: String,
    pub command: String,
    pub working_dir: String,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GitConfig {
    #[serde(default = "default_protected_branches")]
    pub protected_branches: Vec<String>,
    #[serde(default)]
    pub remote_pr: RemotePrConfig,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            protected_branches: default_protected_branches(),
            remote_pr: RemotePrConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemotePrConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_remote_pr_provider")]
    pub provider: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default = "default_remote_pr_api_base")]
    pub api_base: String,
    #[serde(default = "default_remote_pr_token_env")]
    pub token_env: String,
    #[serde(default = "default_remote_pr_base_branch")]
    pub default_base_branch: String,
}

impl Default for RemotePrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_remote_pr_provider(),
            owner: String::new(),
            repo: String::new(),
            api_base: default_remote_pr_api_base(),
            token_env: default_remote_pr_token_env(),
            default_base_branch: default_remote_pr_base_branch(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ModelProfileConfig {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedModelProfile<'a> {
    pub name: &'a str,
    pub profile: &'a ModelProfileConfig,
}

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Yaml(serde_yaml::Error),
    UnsupportedSchema(String),
    InvalidMode(String),
    NoModelProfiles,
    MissingModelProfile(String),
    InvalidModelProfileName(String),
    InvalidModelProfile(String),
    InvalidTestCommand(String),
    DuplicateTestCommand(String),
    InvalidGitConfig(String),
    ProviderNotImplemented(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(
                f,
                "failed to read Gadgets config at {}: {source}. Run `gadgets init` first if this project has not been initialized.",
                path.display()
            ),
            Self::Yaml(err) => write!(f, "failed to parse Gadgets config YAML: {err}"),
            Self::UnsupportedSchema(value) => write!(
                f,
                "unsupported Gadgets config schema version: {value}"
            ),
            Self::InvalidMode(value) => write!(f, "invalid Gadgets runtime mode: {value}"),
            Self::NoModelProfiles => write!(f, "Gadgets config has no model profiles"),
            Self::MissingModelProfile(value) => write!(
                f,
                "configured default model profile does not exist: {value}"
            ),
            Self::InvalidModelProfileName(value) => {
                write!(f, "invalid model profile name: {value}")
            }
            Self::InvalidModelProfile(value) => write!(f, "invalid model profile: {value}"),
            Self::InvalidTestCommand(value) => write!(f, "invalid test command: {value}"),
            Self::DuplicateTestCommand(value) => write!(f, "duplicate test command name: {value}"),
            Self::InvalidGitConfig(value) => write!(f, "invalid git config: {value}"),
            Self::ProviderNotImplemented(value) => write!(
                f,
                "provider `{value}` is configured but not implemented; supported providers are mock, openai, and anthropic"
            ),
        }
    }
}

impl Error for ConfigError {}

pub fn load_project_config(project_root: &Path) -> Result<RuntimeConfig, ConfigError> {
    let path = config_path(project_root);
    let contents = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
        path: path.clone(),
        source,
    })?;
    RuntimeConfig::from_yaml_str(&contents)
}

pub fn config_path(project_root: &Path) -> PathBuf {
    project_root.join(".gadgets").join("config.yaml")
}

pub fn ensure_supported_provider(provider: &str) -> Result<(), ConfigError> {
    if provider == "mock" || provider == "openai" || provider == "anthropic" {
        return Ok(());
    }
    Err(ConfigError::ProviderNotImplemented(provider.to_string()))
}

fn parse_runtime_mode(input: &str) -> Result<RuntimeMode, ConfigError> {
    match input {
        "safe" => Ok(RuntimeMode::Safe),
        "team" => Ok(RuntimeMode::Team),
        "production" => Ok(RuntimeMode::Production),
        other => Err(ConfigError::InvalidMode(other.to_string())),
    }
}

pub fn normalize_config_relative_path(input: &str) -> Result<PathBuf, ConfigError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::InvalidTestCommand(
            "working_dir must not be empty".to_string(),
        ));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(ConfigError::InvalidTestCommand(format!(
            "working_dir {input:?} must be relative"
        )));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(ConfigError::InvalidTestCommand(format!(
                    "working_dir {input:?} must not traverse parents"
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

pub fn valid_test_command_name(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn default_remote_pr_provider() -> String {
    "github".to_string()
}

fn default_remote_pr_api_base() -> String {
    "https://api.github.com".to_string()
}

fn default_remote_pr_token_env() -> String {
    "GITHUB_TOKEN".to_string()
}

fn default_remote_pr_base_branch() -> String {
    "main".to_string()
}

fn default_protected_branches() -> Vec<String> {
    vec![
        "main".to_string(),
        "master".to_string(),
        "trunk".to_string(),
        "production".to_string(),
        "prod".to_string(),
        "release/".to_string(),
    ]
}

fn validate_git_config(config: &GitConfig) -> Result<(), ConfigError> {
    let mut protected = std::collections::BTreeSet::new();
    for branch in &config.protected_branches {
        let value = branch.trim();
        if value != branch || value.is_empty() {
            return Err(ConfigError::InvalidGitConfig(
                "protected branch entries must be non-empty and must not contain leading or trailing whitespace".to_string(),
            ));
        }
        if !protected.insert(branch.clone()) {
            return Err(ConfigError::InvalidGitConfig(format!(
                "duplicate protected branch entry: {branch}"
            )));
        }
        if !valid_protected_branch_pattern(branch) {
            return Err(ConfigError::InvalidGitConfig(format!(
                "invalid protected branch pattern: {branch}"
            )));
        }
    }
    validate_remote_pr_config(&config.remote_pr)?;
    Ok(())
}

fn validate_remote_pr_config(config: &RemotePrConfig) -> Result<(), ConfigError> {
    if config.provider != "github" {
        return Err(ConfigError::InvalidGitConfig(
            "remote_pr.provider must be github".to_string(),
        ));
    }
    if !config.api_base.starts_with("https://") || config.api_base.ends_with('/') {
        return Err(ConfigError::InvalidGitConfig(
            "remote_pr.api_base must be an https URL without a trailing slash".to_string(),
        ));
    }
    if !valid_env_var_name(&config.token_env) {
        return Err(ConfigError::InvalidGitConfig(
            "remote_pr.token_env must be a valid environment variable name".to_string(),
        ));
    }
    if !valid_git_branch_fragment(&config.default_base_branch) {
        return Err(ConfigError::InvalidGitConfig(
            "remote_pr.default_base_branch must be a safe branch name".to_string(),
        ));
    }
    if config.enabled {
        if !valid_remote_repo_component(&config.owner) {
            return Err(ConfigError::InvalidGitConfig(
                "remote_pr.owner must be configured when remote PR creation is enabled".to_string(),
            ));
        }
        if !valid_remote_repo_component(&config.repo) {
            return Err(ConfigError::InvalidGitConfig(
                "remote_pr.repo must be configured when remote PR creation is enabled".to_string(),
            ));
        }
    }
    Ok(())
}

fn valid_remote_repo_component(input: &str) -> bool {
    !input.is_empty()
        && input.trim() == input
        && !input.starts_with('.')
        && !input.ends_with('.')
        && !input.contains("..")
        && input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn valid_env_var_name(input: &str) -> bool {
    !input.is_empty()
        && input.trim() == input
        && input
            .chars()
            .enumerate()
            .all(|(idx, c)| c == '_' || c.is_ascii_uppercase() || (idx > 0 && c.is_ascii_digit()))
}

fn valid_protected_branch_pattern(input: &str) -> bool {
    if let Some(prefix) = input.strip_suffix('/') {
        return !prefix.is_empty() && valid_git_branch_fragment(prefix);
    }
    valid_git_branch_fragment(input)
}

fn valid_git_branch_fragment(input: &str) -> bool {
    if input.is_empty()
        || input.starts_with('-')
        || input.starts_with('/')
        || input.ends_with('/')
        || input.ends_with('.')
        || input.ends_with(".lock")
        || input.contains("..")
        || input.contains("//")
        || input.contains("@{")
        || input.eq_ignore_ascii_case("head")
    {
        return false;
    }

    input.split('/').all(|part| {
        !part.is_empty()
            && !part.starts_with('.')
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    })
}

fn validate_test_commands(commands: &[TestCommandConfig]) -> Result<(), ConfigError> {
    let mut names = std::collections::BTreeSet::new();
    for command in commands {
        let name = command.name.trim();
        if name != command.name || !valid_test_command_name(name) {
            return Err(ConfigError::InvalidTestCommand(format!(
                "test command name {:?} must be non-empty ASCII alphanumeric, underscore, dash, or dot",
                command.name
            )));
        }
        if !names.insert(command.name.clone()) {
            return Err(ConfigError::DuplicateTestCommand(command.name.clone()));
        }
        if command.command.trim().is_empty() {
            return Err(ConfigError::InvalidTestCommand(format!(
                "test command {} has an empty command",
                command.name
            )));
        }
        normalize_config_relative_path(&command.working_dir)?;
        if matches!(command.timeout_seconds, Some(0)) {
            return Err(ConfigError::InvalidTestCommand(format!(
                "test command {} has timeout_seconds 0",
                command.name
            )));
        }
    }
    Ok(())
}

fn valid_profile_name(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_mock_profile() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
"#,
        )
        .unwrap();

        let selected = config.selected_model_profile().unwrap();
        assert_eq!(config.runtime_mode().unwrap(), RuntimeMode::Safe);
        assert_eq!(selected.name, "mock_default");
        assert_eq!(selected.profile.provider, "mock");
        assert_eq!(selected.profile.api_key_env, None);
    }

    #[test]
    fn parses_openai_profile_options() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: openai_default
model_profiles:
  openai_default:
    provider: openai
    model: gpt-5.5
    api_key_env: OPENAI_API_KEY
    endpoint: https://api.openai.com/v1/responses
"#,
        )
        .unwrap();

        let selected = config.selected_model_profile().unwrap();
        assert_eq!(selected.profile.provider, "openai");
        assert_eq!(
            selected.profile.api_key_env.as_deref(),
            Some("OPENAI_API_KEY")
        );
        ensure_supported_provider(&selected.profile.provider).unwrap();
    }

    #[test]
    fn parses_anthropic_profile_options() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: anthropic_default
model_profiles:
  anthropic_default:
    provider: anthropic
    model: claude-sonnet-4-6
    api_key_env: ANTHROPIC_API_KEY
    endpoint: https://api.anthropic.com/v1/messages
"#,
        )
        .unwrap();

        let selected = config.selected_model_profile().unwrap();
        assert_eq!(selected.profile.provider, "anthropic");
        assert_eq!(
            selected.profile.api_key_env.as_deref(),
            Some("ANTHROPIC_API_KEY")
        );
        ensure_supported_provider(&selected.profile.provider).unwrap();
    }

    #[test]
    fn rejects_missing_default_profile() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: missing
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::MissingModelProfile(_)));
    }

    #[test]
    fn rejects_invalid_mode() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: unsafe
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::InvalidMode(_)));
    }

    #[test]
    fn rejects_unknown_provider() {
        assert!(matches!(
            ensure_supported_provider("example-ai"),
            Err(ConfigError::ProviderNotImplemented(_))
        ));
    }

    #[test]
    fn parses_allowlisted_test_commands() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
    timeout_seconds: 300
"#,
        )
        .unwrap();

        assert_eq!(config.test_commands.len(), 1);
        assert_eq!(config.test_commands[0].name, "cargo_test");
        assert_eq!(config.test_commands[0].timeout_seconds, Some(300));
    }

    #[test]
    fn rejects_duplicate_test_command_names() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "."
  - name: cargo_test
    command: cargo check
    working_dir: "."
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::DuplicateTestCommand(_)));
    }

    #[test]
    fn rejects_parent_traversal_test_working_dir() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
test_commands:
  - name: cargo_test
    command: cargo test
    working_dir: "../outside"
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::InvalidTestCommand(_)));
    }

    #[test]
    fn parses_default_git_protected_branches() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
"#,
        )
        .unwrap();

        assert!(config
            .git
            .protected_branches
            .iter()
            .any(|item| item == "main"));
        assert!(config
            .git
            .protected_branches
            .iter()
            .any(|item| item == "release/"));
    }

    #[test]
    fn rejects_invalid_git_protected_branch_pattern() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
git:
  protected_branches:
    - main
    - "../escape"
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::InvalidGitConfig(_)));
    }

    #[test]
    fn parses_disabled_remote_pr_defaults() {
        let config = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
git:
  protected_branches:
    - main
"#,
        )
        .unwrap();

        assert!(!config.git.remote_pr.enabled);
        assert_eq!(config.git.remote_pr.provider, "github");
        assert_eq!(config.git.remote_pr.token_env, "GITHUB_TOKEN");
        assert_eq!(config.git.remote_pr.default_base_branch, "main");
    }

    #[test]
    fn rejects_enabled_remote_pr_without_repo() {
        let err = RuntimeConfig::from_yaml_str(
            r#"schema_version: gadgets.framework/config/v0.1
mode: safe
default_model_profile: mock_default
model_profiles:
  mock_default:
    provider: mock
    model: deterministic-mock
git:
  remote_pr:
    enabled: true
    provider: github
    token_env: GITHUB_TOKEN
    default_base_branch: main
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ConfigError::InvalidGitConfig(_)));
    }
}
