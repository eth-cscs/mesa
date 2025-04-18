use backend_dispatcher::types::cfs::component::{
    Component as FrontEndComponent, ComponentVec as FrontEndComponentVec, State as FrontEndState,
};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/* #[derive(Debug, Serialize, Deserialize, Clone)]
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
} */

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

impl From<FrontEndState> for State {
    fn from(state: FrontEndState) -> Self {
        State {
            clone_url: state.clone_url,
            playbook: state.playbook,
            commit: state.commit,
            session_name: state.session_name,
        }
    }
}

impl Into<FrontEndState> for State {
    fn into(self) -> FrontEndState {
        FrontEndState {
            clone_url: self.clone_url,
            playbook: self.playbook,
            commit: self.commit,
            session_name: self.session_name,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl From<FrontEndComponent> for Component {
    fn from(component: FrontEndComponent) -> Self {
        let mut state_vec = Vec::new();
        for state in component.state.unwrap() {
            let state = State {
                clone_url: state.clone_url,
                playbook: state.playbook,
                commit: state.commit,
                session_name: state.session_name,
            };
            state_vec.push(state);
        }

        /* let state_append = if let Some(state) = component.state_append {
            Some(State {
                clone_url: state.clone_url,
                playbook: state.playbook,
                commit: state.commit,
                session_name: state.session_name,
            })
        } else {
            None
        }; */

        Component {
            id: component.id,
            state: Some(state_vec),
            state_append: None,
            desired_config: component.desired_config,
            error_count: component.error_count,
            retry_policy: component.retry_policy,
            enabled: component.enabled,
            tags: component.tags,
            configuration_status: component.configuration_status,
        }
    }
}

impl Into<FrontEndComponent> for Component {
    fn into(self) -> FrontEndComponent {
        FrontEndComponent {
            id: self.id,
            state: self
                .state
                .map(|state_vec| state_vec.into_iter().map(|state| state.into()).collect()),
            desired_config: self.desired_config,
            error_count: self.error_count,
            retry_policy: self.retry_policy,
            enabled: self.enabled,
            configuration_status: self.configuration_status,
            tags: self.tags,
            logs: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchComponent {
    patch: Vec<Component>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentVec {
    pub components: Vec<Component>,
}

impl From<FrontEndComponentVec> for ComponentVec {
    fn from(component_vec: FrontEndComponentVec) -> Self {
        Self {
            components: component_vec
                .components
                .into_iter()
                .map(|component| component.into())
                .collect(),
        }
    }
}

impl Into<FrontEndComponentVec> for ComponentVec {
    fn into(self) -> FrontEndComponentVec {
        FrontEndComponentVec {
            components: self
                .components
                .into_iter()
                .map(|component| component.into())
                .collect(),
        }
    }
}
