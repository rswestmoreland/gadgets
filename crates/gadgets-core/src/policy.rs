use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub decision_id: String,
    pub action_request_id: String,
    pub decision: DecisionKind,
    pub reason: String,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub matched_rules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Allowed,
    Denied,
    RequiresApproval,
}
