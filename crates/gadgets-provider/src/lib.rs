//! Model provider adapter traits, deterministic mock provider, OpenAI adapter, and Anthropic adapter.
//!
//! Provider adapters translate between model vendors and the Gadgets runtime.
//! They do not authorize tools, execute actions, bypass policy, approve work, or
//! mutate state. Provider output is treated as an untrusted structured request
//! that must still pass runtime handoff, policy, evidence, and audit checks.

use gadgets_core::{HandoffRequest, HandoffScope};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fmt;

pub const CRATE_NAME: &str = "gadgets-provider";
pub const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1/responses";
pub const DEFAULT_OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
pub const DEFAULT_ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
pub const DEFAULT_ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
pub const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    pub request_id: String,
    pub run_id: String,
    pub coordinator_gadget: String,
    pub model_profile: String,
    pub user_prompt: String,
    pub allowed_target_gadgets: Vec<String>,
}

impl ProviderRequest {
    pub fn coordinator_request(
        request_id: impl Into<String>,
        run_id: impl Into<String>,
        user_prompt: impl Into<String>,
        allowed_target_gadgets: Vec<String>,
    ) -> Self {
        Self::coordinator_request_with_profile(
            request_id,
            run_id,
            "mock_default",
            user_prompt,
            allowed_target_gadgets,
        )
    }

    pub fn coordinator_request_with_profile(
        request_id: impl Into<String>,
        run_id: impl Into<String>,
        model_profile: impl Into<String>,
        user_prompt: impl Into<String>,
        allowed_target_gadgets: Vec<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            run_id: run_id.into(),
            coordinator_gadget: "coordinator".to_string(),
            model_profile: model_profile.into(),
            user_prompt: user_prompt.into(),
            allowed_target_gadgets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    pub response_id: String,
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub status: ProviderResponseStatus,
    pub text_summary: String,
    pub handoff_requests: Vec<HandoffRequest>,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderResponseStatus {
    Completed,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    EmptyPrompt,
    NoAllowedTarget(String),
    MissingApiKeyEnv(String),
    Http(String),
    InvalidResponse(String),
    StructuredOutputInvalid(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => write!(f, "provider request prompt is empty"),
            Self::NoAllowedTarget(target) => write!(
                f,
                "provider cannot create handoff because target Gadget is not allowed: {target}"
            ),
            Self::MissingApiKeyEnv(name) => {
                write!(f, "provider requires API key environment variable {name}")
            }
            Self::Http(message) => write!(f, "provider HTTP request failed: {message}"),
            Self::InvalidResponse(message) => write!(f, "provider response was invalid: {message}"),
            Self::StructuredOutputInvalid(message) => {
                write!(f, "provider structured output was invalid: {message}")
            }
        }
    }
}

impl Error for ProviderError {}

pub trait ModelProvider {
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockProvider {
    provider_name: String,
    model_name: String,
}

impl MockProvider {
    pub fn new(provider_name: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            provider_name: provider_name.into(),
            model_name: model_name.into(),
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new("mock", "deterministic-mock")
    }
}

impl ModelProvider for MockProvider {
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.user_prompt.trim().is_empty() {
            return Err(ProviderError::EmptyPrompt);
        }

        let wants_patch_plan = asks_for_patch_plan(&request.user_prompt);
        let target = if wants_patch_plan
            && request
                .allowed_target_gadgets
                .iter()
                .any(|value| value == "patch.writer")
        {
            "patch.writer"
        } else {
            "filesystem.read"
        };

        if !request
            .allowed_target_gadgets
            .iter()
            .any(|value| value == target)
        {
            return Err(ProviderError::NoAllowedTarget(target.to_string()));
        }

        let mut safety_notes = vec![
            "Mock provider produced a structured handoff only.".to_string(),
            "The provider did not read files, execute tools, approve actions, or mutate state."
                .to_string(),
            "Runtime policy still controls every action.".to_string(),
        ];

        if asks_for_mutation(&request.user_prompt) {
            safety_notes.push(
                "The user request appears to mention mutation; the runtime will only run the currently implemented safe slice.".to_string(),
            );
        }

        let (text_summary, handoff) = if target == "patch.writer" {
            (
                format!(
                    "Coordinator plan: request a plan-only Patch Writer proposal for: {}",
                    request.user_prompt
                ),
                patch_plan_handoff(
                    &request.run_id,
                    &request.coordinator_gadget,
                    target,
                    "Create a plan-only proposed patch artifact without modifying files.",
                ),
            )
        } else {
            (
                format!(
                    "Coordinator plan: run an observe-only repository inspection for request: {}",
                    request.user_prompt
                ),
                repo_inspect_handoff(
                    &request.run_id,
                    &request.coordinator_gadget,
                    target,
                    "Inspect the local repository before any planning or changes.",
                ),
            )
        };

        Ok(ProviderResponse {
            response_id: format!("mdlr_{}", request.run_id),
            request_id: request.request_id.clone(),
            provider: self.provider_name.clone(),
            model: self.model_name.clone(),
            status: ProviderResponseStatus::Completed,
            text_summary,
            handoff_requests: vec![handoff],
            safety_notes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiProvider {
    model_name: String,
    api_key: String,
    endpoint: String,
}

impl OpenAiProvider {
    pub fn from_env(
        model_name: impl Into<String>,
        api_key_env: Option<&str>,
        endpoint: Option<&str>,
    ) -> Result<Self, ProviderError> {
        let api_key_env = api_key_env.unwrap_or(DEFAULT_OPENAI_API_KEY_ENV);
        let api_key = env::var(api_key_env)
            .map_err(|_| ProviderError::MissingApiKeyEnv(api_key_env.to_string()))?;
        Ok(Self::new(
            model_name,
            api_key,
            endpoint.unwrap_or(DEFAULT_OPENAI_ENDPOINT),
        ))
    }

    pub fn new(
        model_name: impl Into<String>,
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            api_key: api_key.into(),
            endpoint: endpoint.into(),
        }
    }

    fn build_body(&self, request: &ProviderRequest) -> Value {
        json!({
            "model": self.model_name.clone(),
            "store": false,
            "tools": [],
            "input": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "input_text",
                            "text": coordinator_system_prompt(&request.allowed_target_gadgets)
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": request.user_prompt.clone()
                        }
                    ]
                }
            ],
            "text": {
                "format": coordinator_json_schema()
            }
        })
    }
}

impl ModelProvider for OpenAiProvider {
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.user_prompt.trim().is_empty() {
            return Err(ProviderError::EmptyPrompt);
        }

        let body = self.build_body(request);
        let response_text = ureq::post(&self.endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_string(&body.to_string())
            .map_err(|err| ProviderError::Http(err.to_string()))?
            .into_string()
            .map_err(|err| ProviderError::Http(err.to_string()))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
        let output_text = extract_output_text(&response_json)?;
        let model_output: CoordinatorStructuredOutput = serde_json::from_str(&output_text)
            .map_err(|err| ProviderError::StructuredOutputInvalid(err.to_string()))?;

        let handoffs = model_output
            .handoff_requests
            .into_iter()
            .map(|handoff| handoff.into_handoff_request(request))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ProviderResponse {
            response_id: response_json
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("openai_response")
                .to_string(),
            request_id: request.request_id.clone(),
            provider: "openai".to_string(),
            model: response_json
                .get("model")
                .and_then(Value::as_str)
                .map_or_else(|| self.model_name.clone(), |value| value.to_string()),
            status: ProviderResponseStatus::Completed,
            text_summary: model_output.text_summary,
            handoff_requests: handoffs,
            safety_notes: normalize_safety_notes(model_output.safety_notes),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicProvider {
    model_name: String,
    api_key: String,
    endpoint: String,
    anthropic_version: String,
}

impl AnthropicProvider {
    pub fn from_env(
        model_name: impl Into<String>,
        api_key_env: Option<&str>,
        endpoint: Option<&str>,
    ) -> Result<Self, ProviderError> {
        let api_key_env = api_key_env.unwrap_or(DEFAULT_ANTHROPIC_API_KEY_ENV);
        let api_key = env::var(api_key_env)
            .map_err(|_| ProviderError::MissingApiKeyEnv(api_key_env.to_string()))?;
        Ok(Self::new(
            model_name,
            api_key,
            endpoint.unwrap_or(DEFAULT_ANTHROPIC_ENDPOINT),
            DEFAULT_ANTHROPIC_VERSION,
        ))
    }

    pub fn new(
        model_name: impl Into<String>,
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
        anthropic_version: impl Into<String>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            api_key: api_key.into(),
            endpoint: endpoint.into(),
            anthropic_version: anthropic_version.into(),
        }
    }

    fn build_body(&self, request: &ProviderRequest) -> Value {
        json!({
            "model": self.model_name.clone(),
            "max_tokens": 2048,
            "system": coordinator_system_prompt(&request.allowed_target_gadgets),
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": request.user_prompt.clone()
                        }
                    ]
                }
            ],
            "tools": [],
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": coordinator_schema_definition()
                }
            }
        })
    }
}

impl ModelProvider for AnthropicProvider {
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.user_prompt.trim().is_empty() {
            return Err(ProviderError::EmptyPrompt);
        }

        let body = self.build_body(request);
        let response_text = ureq::post(&self.endpoint)
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", &self.anthropic_version)
            .send_string(&body.to_string())
            .map_err(|err| ProviderError::Http(err.to_string()))?
            .into_string()
            .map_err(|err| ProviderError::Http(err.to_string()))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;

        if response_json.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
            return Ok(ProviderResponse {
                response_id: response_json
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("anthropic_response")
                    .to_string(),
                request_id: request.request_id.clone(),
                provider: "anthropic".to_string(),
                model: response_json
                    .get("model")
                    .and_then(Value::as_str)
                    .map_or_else(|| self.model_name.clone(), |value| value.to_string()),
                status: ProviderResponseStatus::Refused,
                text_summary: "Anthropic provider returned a refusal.".to_string(),
                handoff_requests: Vec::new(),
                safety_notes: vec![
                    "Provider refusal produced no handoff; runtime took no action.".to_string(),
                ],
            });
        }

        if response_json.get("stop_reason").and_then(Value::as_str) == Some("max_tokens") {
            return Err(ProviderError::InvalidResponse(
                "Anthropic response stopped at max_tokens before complete structured output"
                    .to_string(),
            ));
        }

        let output_text = extract_anthropic_text(&response_json)?;
        let model_output: CoordinatorStructuredOutput = serde_json::from_str(&output_text)
            .map_err(|err| ProviderError::StructuredOutputInvalid(err.to_string()))?;

        let handoffs = model_output
            .handoff_requests
            .into_iter()
            .map(|handoff| handoff.into_handoff_request(request))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ProviderResponse {
            response_id: response_json
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("anthropic_response")
                .to_string(),
            request_id: request.request_id.clone(),
            provider: "anthropic".to_string(),
            model: response_json
                .get("model")
                .and_then(Value::as_str)
                .map_or_else(|| self.model_name.clone(), |value| value.to_string()),
            status: ProviderResponseStatus::Completed,
            text_summary: model_output.text_summary,
            handoff_requests: handoffs,
            safety_notes: normalize_safety_notes(model_output.safety_notes),
        })
    }
}

#[derive(Debug, Deserialize)]
struct CoordinatorStructuredOutput {
    text_summary: String,
    handoff_requests: Vec<ModelHandoffOutput>,
    safety_notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModelHandoffOutput {
    to_gadget: String,
    reason: String,
    task_kind: String,
    zone: String,
    paths: Vec<String>,
    required_evidence: Vec<String>,
}

impl ModelHandoffOutput {
    fn into_handoff_request(
        self,
        request: &ProviderRequest,
    ) -> Result<HandoffRequest, ProviderError> {
        if !request
            .allowed_target_gadgets
            .iter()
            .any(|value| value == &self.to_gadget)
        {
            return Err(ProviderError::NoAllowedTarget(self.to_gadget));
        }
        if self.task_kind.trim().is_empty() {
            return Err(ProviderError::StructuredOutputInvalid(
                "handoff task_kind is empty".to_string(),
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(ProviderError::StructuredOutputInvalid(
                "handoff reason is empty".to_string(),
            ));
        }

        Ok(HandoffRequest {
            handoff_id: format!("hnd_{}", request.run_id),
            from_gadget: request.coordinator_gadget.clone(),
            to_gadget: self.to_gadget,
            reason: self.reason,
            task_kind: self.task_kind,
            scope: HandoffScope {
                zone: Some(self.zone),
                paths: self.paths,
            },
            required_evidence: self.required_evidence,
        })
    }
}

fn repo_inspect_handoff(
    run_id: &str,
    from_gadget: &str,
    to_gadget: &str,
    reason: &str,
) -> HandoffRequest {
    HandoffRequest {
        handoff_id: format!("hnd_{run_id}"),
        from_gadget: from_gadget.to_string(),
        to_gadget: to_gadget.to_string(),
        reason: reason.to_string(),
        task_kind: "repo.inspect".to_string(),
        scope: HandoffScope {
            zone: Some("local_repo".to_string()),
            paths: vec![".".to_string()],
        },
        required_evidence: vec![
            "summary".to_string(),
            "files_read".to_string(),
            "denied_accesses".to_string(),
            "assumptions".to_string(),
        ],
    }
}

fn patch_plan_handoff(
    run_id: &str,
    from_gadget: &str,
    to_gadget: &str,
    reason: &str,
) -> HandoffRequest {
    HandoffRequest {
        handoff_id: format!("hnd_{run_id}"),
        from_gadget: from_gadget.to_string(),
        to_gadget: to_gadget.to_string(),
        reason: reason.to_string(),
        task_kind: "repo.patch.plan".to_string(),
        scope: HandoffScope {
            zone: Some("local_repo".to_string()),
            paths: vec!["proposed.patch".to_string()],
        },
        required_evidence: vec![
            "summary".to_string(),
            "diff".to_string(),
            "assumptions".to_string(),
        ],
    }
}

fn coordinator_system_prompt(allowed_targets: &[String]) -> String {
    format!(
        "You are the Coordinator Gadget for Gadgets Framework. You may reason and propose a structured handoff, but you cannot read files, execute tools, approve actions, or mutate state. Only the Gadgets runtime may authorize and execute actions. Allowed target Gadgets: {}. Choose filesystem.read with task_kind repo.inspect for repository inspection. Choose patch.writer with task_kind repo.patch.plan only when the user asks for a patch, code change, tests, docs change, or fix; this is plan-only and must not claim files were modified. Return only valid structured JSON matching the supplied schema.",
        allowed_targets.join(", ")
    )
}

fn coordinator_json_schema() -> Value {
    json!({
        "type": "json_schema",
        "name": "gadgets_coordinator_handoff",
        "strict": true,
        "schema": coordinator_schema_definition()
    })
}

fn coordinator_schema_definition() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "text_summary": {
                "type": "string",
                "description": "A concise user-facing summary of the proposed safe plan."
            },
            "handoff_requests": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "to_gadget": { "type": "string" },
                        "reason": { "type": "string" },
                        "task_kind": { "type": "string" },
                        "zone": { "type": "string" },
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "required_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": [
                        "to_gadget",
                        "reason",
                        "task_kind",
                        "zone",
                        "paths",
                        "required_evidence"
                    ]
                }
            },
            "safety_notes": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["text_summary", "handoff_requests", "safety_notes"]
    })
}

fn extract_output_text(response_json: &Value) -> Result<String, ProviderError> {
    let output = response_json
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::InvalidResponse("missing output array".to_string()))?;

    for item in output {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            if part.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    return Ok(text.to_string());
                }
            }
        }
    }

    Err(ProviderError::InvalidResponse(
        "missing output_text content".to_string(),
    ))
}

fn extract_anthropic_text(response_json: &Value) -> Result<String, ProviderError> {
    let content = response_json
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::InvalidResponse("missing content array".to_string()))?;

    for part in content {
        if part.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                return Ok(text.to_string());
            }
        }
    }

    Err(ProviderError::InvalidResponse(
        "missing text content".to_string(),
    ))
}

fn normalize_safety_notes(mut notes: Vec<String>) -> Vec<String> {
    notes.push("Provider output was treated as an untrusted structured request.".to_string());
    notes.push("Runtime policy still controls every action.".to_string());
    notes
}

fn asks_for_patch_plan(prompt: &str) -> bool {
    prompt_has_any_word(
        prompt,
        &[
            "patch", "diff", "fix", "change", "edit", "add", "update", "write", "test", "tests",
            "doc", "docs", "readme",
        ],
    )
}

fn asks_for_mutation(prompt: &str) -> bool {
    prompt_has_any_word(
        prompt,
        &[
            "write", "change", "edit", "delete", "remove", "fix", "patch", "install", "restart",
            "deploy", "apply",
        ],
    )
}

fn prompt_has_any_word(prompt: &str, words: &[&str]) -> bool {
    prompt
        .to_ascii_lowercase()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|token| words.contains(&token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_returns_filesystem_read_handoff() {
        let provider = MockProvider::default();
        let request = ProviderRequest::coordinator_request(
            "mdl_1",
            "run_1",
            "Review this repo",
            vec!["filesystem.read".to_string()],
        );

        let response = provider.complete(&request).unwrap();

        assert_eq!(response.provider, "mock");
        assert_eq!(response.handoff_requests.len(), 1);
        assert_eq!(response.handoff_requests[0].to_gadget, "filesystem.read");
        assert_eq!(response.handoff_requests[0].task_kind, "repo.inspect");
        assert_eq!(
            response.handoff_requests[0].scope.zone.as_deref(),
            Some("local_repo")
        );
    }

    #[test]
    fn mock_provider_returns_patch_writer_plan_handoff_for_change_request() {
        let provider = MockProvider::default();
        let request = ProviderRequest::coordinator_request(
            "mdl_1",
            "run_1",
            "Add parser tests",
            vec!["filesystem.read".to_string(), "patch.writer".to_string()],
        );

        let response = provider.complete(&request).unwrap();

        assert_eq!(response.provider, "mock");
        assert_eq!(response.handoff_requests.len(), 1);
        assert_eq!(response.handoff_requests[0].to_gadget, "patch.writer");
        assert_eq!(response.handoff_requests[0].task_kind, "repo.patch.plan");
        assert_eq!(
            response.handoff_requests[0].scope.zone.as_deref(),
            Some("local_repo")
        );
    }

    #[test]
    fn mock_provider_rejects_unavailable_target() {
        let provider = MockProvider::default();
        let request = ProviderRequest::coordinator_request(
            "mdl_1",
            "run_1",
            "Review this repo",
            vec!["documentation.writer".to_string()],
        );

        assert!(matches!(
            provider.complete(&request),
            Err(ProviderError::NoAllowedTarget(_))
        ));
    }

    #[test]
    fn openai_provider_requires_api_key_env() {
        let env_name = "GADGETS_TEST_OPENAI_KEY_NOT_SET";
        std::env::remove_var(env_name);
        let provider = OpenAiProvider::from_env("gpt-5.5", Some(env_name), None);
        assert!(matches!(provider, Err(ProviderError::MissingApiKeyEnv(_))));
    }

    #[test]
    fn extracts_output_text_from_responses_api_shape() {
        let response = json!({
            "id": "resp_1",
            "model": "gpt-test",
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "{\"text_summary\":\"ok\",\"handoff_requests\":[],\"safety_notes\":[]}"
                        }
                    ]
                }
            ]
        });

        let text = extract_output_text(&response).unwrap();
        assert!(text.contains("text_summary"));
    }

    #[test]
    fn anthropic_provider_requires_api_key_env() {
        let env_name = "GADGETS_TEST_ANTHROPIC_KEY_NOT_SET";
        std::env::remove_var(env_name);
        let provider = AnthropicProvider::from_env("claude-sonnet-4-6", Some(env_name), None);
        assert!(matches!(provider, Err(ProviderError::MissingApiKeyEnv(_))));
    }

    #[test]
    fn extracts_text_from_anthropic_messages_shape() {
        let response = json!({
            "id": "msg_1",
            "model": "claude-test",
            "content": [
                {
                    "type": "text",
                    "text": "{\"text_summary\":\"ok\",\"handoff_requests\":[],\"safety_notes\":[]}"
                }
            ],
            "stop_reason": "end_turn"
        });

        let text = extract_anthropic_text(&response).unwrap();
        assert!(text.contains("text_summary"));
    }

    #[test]
    fn anthropic_body_uses_messages_api_structured_outputs() {
        let provider = AnthropicProvider::new(
            "claude-sonnet-4-6",
            "test-key",
            DEFAULT_ANTHROPIC_ENDPOINT,
            DEFAULT_ANTHROPIC_VERSION,
        );
        let request = ProviderRequest::coordinator_request(
            "mdl_1",
            "run_1",
            "Review this repo",
            vec!["filesystem.read".to_string()],
        );
        let body = provider.build_body(&request);

        assert_eq!(
            body.get("model").and_then(Value::as_str),
            Some("claude-sonnet-4-6")
        );
        assert!(body.get("system").is_some());
        assert!(body.get("messages").is_some());
        assert!(body.get("output_config").is_some());
    }

    #[test]
    fn rejects_structured_handoff_to_unallowed_target() {
        let output = ModelHandoffOutput {
            to_gadget: "deployment.executor".to_string(),
            reason: "bad".to_string(),
            task_kind: "deploy".to_string(),
            zone: "production_environment".to_string(),
            paths: vec![],
            required_evidence: vec![],
        };
        let request = ProviderRequest::coordinator_request(
            "mdl_1",
            "run_1",
            "deploy",
            vec!["filesystem.read".to_string()],
        );

        assert!(matches!(
            output.into_handoff_request(&request),
            Err(ProviderError::NoAllowedTarget(_))
        ));
    }
}
