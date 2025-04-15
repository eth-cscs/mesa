// TODO: Update/Review these structs because:
// - PUT/PATH operations are tricky since some fields are read-only
// - State.lastUpdate field may be missing
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use backend_dispatcher::types::cfs::component::{
    Component as FrontEndComponent, ComponentVec as FrontEndComponentVec, State as FrontEndState,
};

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

impl From<FrontEndComponent> for Component {
    fn from(component: FrontEndComponent) -> Self {
        Component {
            id: component.id,
            state: component
                .state
                .map(|state_vec| state_vec.into_iter().map(|state| state.into()).collect()),
            desired_config: component.desired_config,
            error_count: component.error_count,
            retry_policy: component.retry_policy,
            enabled: component.enabled,
            configuration_status: component.configuration_status,
            tags: component.tags,
            logs: component.logs,
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
            logs: self.logs,
        }
    }
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
