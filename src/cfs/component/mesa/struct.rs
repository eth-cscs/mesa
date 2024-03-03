use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ConfigurationStateLayer {
    #[serde(rename = "cloneUrl")]
    pub clone_url: String,
    pub playbook: String,
    pub commit: String,
    #[serde(rename = "sessionName")]
    pub session_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CfsComponent {
    pub id: String,
    pub state: Vec<ConfigurationStateLayer>,
    #[serde(rename = "stateAppend")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_append: Option<ConfigurationStateLayer>,
    #[serde(rename = "desiredConfig")]
    pub desired_config: String,
    #[serde(rename = "desiredState")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_state: Option<Vec<ConfigurationStateLayer>>, // this field is missing in request payload
    // when creation a new CFS component
    #[serde(rename = "errorCount")]
    pub error_count: u32,
    #[serde(rename = "retryPolicy")]
    #[serde(default)]
    pub retry_policy: u32,
    pub enabled: bool,
    #[serde(rename = "configurationStatus")]
    pub configuration_status: String, //values unconfigured, pending, failed, configured
    pub tags: HashMap<String, String>,
}
