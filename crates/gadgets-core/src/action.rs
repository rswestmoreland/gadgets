use serde::{Deserialize, Serialize};

use crate::capability::CapabilityName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action_request_id: String,
    pub run_id: String,
    pub requested_by_gadget: String,
    pub capability: CapabilityName,
    pub tool: String,
    pub target: ActionTarget,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActionTarget {
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
}
