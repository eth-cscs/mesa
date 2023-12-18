use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SshContainer {
    pub name: String,
    pub jail: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Job {
    pub job_type: String,
    pub image_root_archive_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_parameters_file_name: Option<String>,
    pub artifact_id: String,
    pub public_key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_containers: Option<Vec<SshContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buid_env_size: Option<u8>,
}
