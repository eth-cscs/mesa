// TODO: Update/Review these structs because:
// - PUT/PATH operations are tricky since some fields are read-only
// - State.lastUpdate field may be missing
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<State>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_status: Option<String>, //values unconfigured, pending, failed, configured
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentVec {
    pub components: Vec<Component>,
}
