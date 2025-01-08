use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SshContainer {
    pub name: String,
    pub jail: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct JobPostRequest {
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
    pub build_env_size: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct JobGetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    pub job_type: String,
    pub image_root_archive_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_parameters_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub artifact_id: String,
    pub public_key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_configmap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_containers: Option<Vec<SshContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resultant_image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_env_size: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_namespace: Option<String>,
}
