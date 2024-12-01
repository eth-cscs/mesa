use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cloneUrl")]
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sesisonName")]
    pub session_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<StateResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stateAppend")]
    pub state_append: Option<StateResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "desiredConfig")]
    pub desired_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "errorCount")]
    pub error_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryPolicy")]
    pub retry_policy: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "configurationStatus")]
    pub configuration_status: Option<String>, //values unconfigured, pending, failed, configured
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cloneUrl")]
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sesisonName")]
    pub session_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<StateRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stateAppend")]
    pub state_append: Option<StateRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "desiredConfig")]
    pub desired_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "errorCount")]
    pub error_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryPolicy")]
    pub retry_policy: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}

impl From<ComponentResponse> for ComponentRequest {
    fn from(component: ComponentResponse) -> Self {
        let mut state_vec = Vec::new();
        for state in component.state.unwrap() {
            let state = StateRequest {
                clone_url: state.clone_url,
                playbook: state.playbook,
                commit: state.commit,
                session_name: state.session_name,
            };
            state_vec.push(state);
        }

        let state_append = if let Some(state) = component.state_append {
            Some(StateRequest {
                clone_url: state.clone_url,
                playbook: state.playbook,
                commit: state.commit,
                session_name: state.session_name,
            })
        } else {
            None
        };

        ComponentRequest {
            id: component.id,
            state: Some(state_vec),
            state_append,
            desired_config: component.desired_config,
            error_count: component.error_count,
            retry_policy: component.retry_policy,
            enabled: component.enabled,
            tags: component.tags,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchComponent {
    patch: Vec<ComponentRequest>,
    filters: Filter,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Filter {
    #[serde(skip_serializing_if = "Option::is_none")]
    ids: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>, // TODO: change to enum
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "configurationName")]
    config_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}
