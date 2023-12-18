use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
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
pub struct Component {
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<State>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stateAppend")]
    pub state_append: Option<State>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "desiredConfig")]
    pub desired_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "errorCount")]
    pub error_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryPolicy")]
    pub retry_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    // tags: TODO: this is supposed to be an object??? https://csm12-apidocs.svc.cscs.ch/paas/cfs/operation/patch_component/#!path=tags&t=request
}
