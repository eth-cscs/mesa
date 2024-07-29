use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDetails {
    pub xname: String,
    pub nid: String,
    pub power_status: String,
    pub desired_configuration: String,
    pub configuration_status: String,
    pub enabled: String,
    pub error_count: String,
    pub boot_image_id: String,
    pub boot_configuration: String,
    pub kernel_params: String,
}
