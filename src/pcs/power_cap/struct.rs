use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerCapTaskList {
    pub tasks: Vec<PowerCapTaskInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskCounts {
    pub total: usize,
    pub new: usize,
    pub in_progress: usize,
    pub failed: usize,
    pub succeeded: usize,
    pub un_supported: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Limit {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hostsLimitMax")]
    pub hosts_limit_max: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hostsLimitMin")]
    pub hosts_limit_min: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "powerupPower")]
    pub powerup_power: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerCapLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "currentValue")]
    pub current_value: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mamximumValue")]
    pub maximum_value: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "minimumValue")]
    pub mnimum_value: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerCapComponent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "power_cap_limits")]
    pub power_cap_limits: Option<PowerCapLimit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerCapTaskInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>, // TODO: convert to enum. Valid values are `snapshot` and `patch`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskCreateTime")]
    pub task_create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "automaticExpirationTime")]
    pub automatic_expiration_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskStatus")]
    pub task_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskCounts")]
    pub task_counts: Option<TaskCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<PowerCapComponent>>,
}
