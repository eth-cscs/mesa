use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PostRequestPayload {
    pub name: String,
    #[serde(rename = "configurationName")]
    pub configuration_name: String,
    #[serde(rename = "configurationLimit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_limit: Option<String>,
    #[serde(rename = "ansibleLimit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_limit: Option<String>,
    #[serde(rename = "ansibleConfig")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_config: Option<String>,
    #[serde(rename = "ansibleVerbosity")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_verbosity: Option<u64>,
    #[serde(rename = "ansiblePassthrough")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_passthrough: Option<String>,
    #[serde(default)]
    pub target: Target,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Target {
    pub definition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Group>>,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            definition: String::from("dynamic"),
            groups: None,
        }
    }
}

impl PostRequestPayload {
    pub fn new(
        name: String,
        configuration_name: String,
        ansible_limit: Option<String>,
        ansible_verbosity: Option<u64>,
        ansible_passthrough: Option<String>,
        groups_name_opt: Option<Vec<String>>,
        groups_members_opt: Option<Vec<Vec<String>>>, // This value is the base image id when building an image
    ) -> Self {
        // This code is fine... the fact that I put Self behind a variable is ok, since image param
        // is not a default param, then doing things differently is not an issue. I checked with
        // other Rust developers in their discord https://discord.com/channels/442252698964721669/448238009733742612/1081686300182188207
        let mut cfs_session = Self {
            name,
            configuration_name,
            ansible_limit,
            ansible_verbosity,
            ansible_passthrough,
            ..Default::default()
        };

        let mut group_vec = Vec::new();

        if let (Some(groups_name), Some(groups_members)) = (groups_name_opt, groups_members_opt) {
            for (group_name, group_members) in groups_name.iter().zip(groups_members.iter()) {
                let group = Group {
                    name: group_name.to_string(),
                    members: group_members.to_vec(),
                };

                group_vec.push(group);
            }
        }

        /* let target_groups: Vec<Group> = groups_name
        .unwrap()
        .into_iter()
        .map(|group_name| Group {
            name: group_name,
            members: vec![base_image_id.as_ref().unwrap().to_string()],
        })
        .collect(); */

        cfs_session.target.definition = "image".to_string();
        cfs_session.target.groups = Some(group_vec);

        cfs_session
    }

    pub fn from_sat_file_serde_yaml(session_yaml: &serde_yaml::Value) -> Self {
        let groups_name = session_yaml["configuration_group_names"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|group_name| group_name.as_str().unwrap().to_string())
            .collect();

        let cfs_session = PostRequestPayload::new(
            session_yaml["name"].as_str().unwrap().to_string(),
            session_yaml["configuration"].as_str().unwrap().to_string(),
            None,
            None,
            None,
            Some(groups_name),
            // Some(base_image_id.to_string()),
            Some(vec![vec![session_yaml["ims"]["id"]
                .as_str()
                .unwrap()
                .to_string()]]),
        );

        cfs_session
    }

    pub fn from_csm_api_json(session_value: Value) -> Self {
        let mut group_vec = Vec::new();

        if let Some(group_value_vec) = session_value.pointer("/target/groups") {
            for group_value in group_value_vec.as_array().unwrap() {
                let group = Group {
                    name: group_value["name"].as_str().unwrap().to_string(),
                    members: group_value["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|member_value| member_value.as_str().unwrap().to_string())
                        .collect(),
                };

                group_vec.push(group);
            }
        }

        let target = Target {
            definition: session_value
                .pointer("/target/definition")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap()
                .to_string(),
            groups: Some(group_vec),
        };

        let mut tag_vec = Vec::new();

        if let Some(tag_value_vec) = session_value.get("tags") {
            for (tag_name, tag_value) in tag_value_vec.as_object().unwrap() {
                let tag = Tag {
                    key: tag_name.to_string(),
                    value: tag_value.as_str().unwrap().to_string(),
                };

                tag_vec.push(tag);
            }
        }

        let session = PostRequestPayload {
            name: session_value["name"].as_str().unwrap().to_string(),
            configuration_name: session_value
                .get("configurationName")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            configuration_limit: session_value
                .get("configurationLimit")
                .unwrap()
                .as_str()
                .map(|str| str.to_string()),
            ansible_limit: session_value
                .get("ansibleLimit")
                .unwrap()
                .as_str()
                .map(|str| str.to_string()),
            ansible_config: session_value
                .get("ansibleConfig")
                .unwrap()
                .as_str()
                .map(|str| str.to_string()),
            ansible_verbosity: session_value.get("ansibleVerbosity").unwrap().as_u64(),
            ansible_passthrough: session_value
                .get("ansibleLimit")
                .unwrap()
                .as_str()
                .map(|str| str.to_string()),
            target,
            tags: Some(tag_vec),
        };

        session
    }
}
