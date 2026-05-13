use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffRequest {
    pub handoff_id: String,
    pub from_gadget: String,
    pub to_gadget: String,
    pub reason: String,
    pub task_kind: String,
    pub scope: HandoffScope,
    #[serde(default)]
    pub required_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HandoffScope {
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
}
