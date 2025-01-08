use serde::{Deserialize, Serialize};

use crate::pcs::transitions::types::Operation;

#[derive(Debug, Serialize, Deserialize)]
pub enum PowerState {
    On,
    Off,
    Undefined,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ManagementState {
    Unavailable,
    Available,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerStatus {
    xname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    power_state_filter: Option<PowerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "powerState")]
    power_state: Option<PowerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "management_state")]
    management_state: Option<ManagementState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    management_state_filter: Option<ManagementState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "supportedPowerTransitions")]
    supported_power_transitions: Option<Operation>,
    last_updated: String,
}
