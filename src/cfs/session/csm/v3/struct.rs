use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfsSessionGetResponseList {
    pub sessions: Vec<CfsSessionGetResponse>,
    pub next: Option<Next>,
}

#[derive(Debug, Serialize, Deserialize, Clone)] // TODO: investigate why serde can Deserialize dynamically syzed structs `Vec<Layer>`
pub struct Next {
    limit: Option<u8>,
    after_id: Option<String>,
    in_use: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfsSessionGetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<Configuration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible: Option<Ansible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
    pub debug_on_failure: bool,
    pub logs: Option<String>,
}

impl CfsSessionGetResponse {
    /// Get start time
    pub fn get_start_time(&self) -> Option<String> {
        self.status.as_ref().and_then(|status| {
            status
                .session
                .as_ref()
                .and_then(|session| session.start_time.clone())
        })
    }

    /// Returns list of result_ids
    pub fn get_result_id_vec(&self) -> Vec<String> {
        if let Some(status) = &self.status {
            status
                .artifacts
                .as_ref()
                .unwrap_or(&Vec::new())
                .into_iter()
                .filter(|artifact| artifact.result_id.is_some())
                .map(|artifact| artifact.result_id.clone().unwrap())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns list of result_ids
    pub fn get_first_result_id(&self) -> Option<String> {
        CfsSessionGetResponse::get_result_id_vec(&self)
            .first()
            .cloned()
    }

    /* /// Returns list of result_ids
    pub fn get_result_id(&self) -> Option<String> {
        self.status.as_ref().and_then(|status| {
            status.artifacts.as_ref().and_then(|artifacts| {
                artifacts
                    .first()
                    .and_then(|artifact| artifact.result_id.clone())
            })
        })
    } */

    /// Returns list of targets (either groups or xnames)
    pub fn get_targets(&self) -> Option<Vec<String>> {
        Some(
            self.get_target_hsm()
                .unwrap_or(self.get_target_xname().unwrap()),
        )
    }

    /// Returns list of HSM groups targeted
    pub fn get_target_hsm(&self) -> Option<Vec<String>> {
        self.target.as_ref().and_then(|target| {
            target
                .groups
                .as_ref()
                .map(|group_vec| group_vec.iter().map(|group| group.name.clone()).collect())
        })
    }

    /// Returns list of xnames targeted
    pub fn get_target_xname(&self) -> Option<Vec<String>> {
        self.ansible.as_ref().and_then(|ansible| {
            ansible.limit.as_ref().map(|limit| {
                limit
                    .split(',')
                    .map(|xname| xname.trim().to_string())
                    .collect()
            })
        })
    }

    /// Returns 'true' if the CFS session target definition is 'image'. Otherwise (target
    /// definiton dynamic) will return 'false'
    pub fn is_target_def_image(&self) -> bool {
        self.get_target_def()
            .is_some_and(|target_def| target_def == "image")
    }

    /// Returns target definition of the CFS session:
    /// image --> CFS session to build an image
    /// dynamic --> CFS session to configure a node
    pub fn get_target_def(&self) -> Option<String> {
        self.target
            .as_ref()
            .and_then(|target| target.definition.clone())
    }

    pub fn get_configuration_name(&self) -> Option<String> {
        self.configuration
            .as_ref()
            .and_then(|configuration| configuration.name.clone())
    }

    /// Returns 'true' if CFS session succeeded
    pub fn is_success(&self) -> bool {
        self.status
            .as_ref()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .succeeded
            .as_ref()
            .unwrap()
            == "true"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ansible {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passthrough: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ims_job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CfsSessionPostRequest {
    pub name: String,
    pub configuration_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_verbosity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_passthrough: Option<String>,
    #[serde(default)]
    pub target: Target,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
    pub debug_on_failure: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Target {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Group>>,
    pub image_map: Option<Vec<ImageMap>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageMap {
    source_id: String,
    result_name: String,
}

impl CfsSessionPostRequest {
    pub fn new(
        name: String,
        configuration_name: String,
        configuration_limit: Option<String>,
        ansible_limit: Option<String>,
        ansible_config: Option<String>,
        ansible_verbosity: Option<u8>,
        ansible_passthrough: Option<String>,
        is_target_definition_image: bool,
        groups_name: Option<Vec<String>>,
        base_image_id: Option<String>,
        tags: Option<HashMap<String, String>>,
        debug_on_failure: bool,
        result_image_name: Option<String>,
    ) -> Self {
        // This code is fine... the fact that I put Self behind a variable is ok, since image param
        // is not a default param, then doing things differently is not an issue. I checked with
        // other Rust developers in their discord https://discord.com/channels/442252698964721669/448238009733742612/1081686300182188207
        let mut cfs_session = Self {
            name,
            configuration_name,
            configuration_limit,
            ansible_config,
            ansible_limit,
            ansible_verbosity,
            ansible_passthrough,
            ..Default::default()
        };

        if is_target_definition_image {
            let target_groups: Vec<Group> = groups_name
                .unwrap()
                .into_iter()
                .map(|group_name| Group {
                    name: group_name,
                    members: vec![base_image_id.as_ref().unwrap().to_string()],
                })
                .collect();

            cfs_session.target.definition = Some("image".to_string());
            cfs_session.target.groups = Some(target_groups);
            cfs_session.target.image_map = Some(vec![ImageMap {
                            source_id: base_image_id.expect("ERROR - can't create a CFS session to build an image without base image id"),
                            result_name: result_image_name.expect("ERROR - can't create a CFS sessions to build an image without result image name"),
                        }]);
        } else {
            cfs_session.target.definition = Some("dynamic".to_string());
            cfs_session.target.groups = None;
            cfs_session.target.image_map = Some(Vec::new());
        }

        cfs_session.tags = tags;
        cfs_session.debug_on_failure = debug_on_failure;

        cfs_session
    }
}
