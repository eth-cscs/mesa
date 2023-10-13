use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetResponse {
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
    pub tags: Option<Vec<Tag>>,
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
pub struct Target {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Group>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
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
    #[serde(rename = "completionTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_time: Option<String>,
    #[serde(rename = "startTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl GetResponse {
    pub fn from_csm_api_json(session_value: Value) -> Self {
        let configuration = Configuration {
            name: session_value
                .pointer("/configuration/name")
                .and_then(|value| Some(value.as_str().unwrap_or("").to_string())),
            limit: session_value
                .pointer("/configuration/limit")
                .and_then(|value| Some(value.as_str().unwrap_or("").to_string())),
        };

        let ansible = Ansible {
            config: session_value
                .pointer("/ansible/config")
                .and_then(|value| Some(value.as_str().unwrap_or("").to_string())),
            limit: session_value
                .pointer("/ansible/limit")
                .and_then(|value| Some(value.as_str().unwrap_or("").to_string())),
            verbosity: session_value
                .pointer("/ansible/verbosity")
                .and_then(|str| Some(str.as_u64().unwrap())),
            passthrough: session_value
                .pointer("/ansible/passthrough")
                .and_then(|value| Some(value.as_str().unwrap_or("").to_string())),
        };

        let mut group_vec = Vec::new();

        if let Some(group_vec_value) = session_value.pointer("/target/groups") {
            for group_value in group_vec_value.as_array().unwrap_or(&Vec::new()) {
                let group = Group {
                    name: group_value["name"].as_str().map(|str| str.to_string()),
                    members: Some(
                        group_value["members"]
                            .as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|str| str.to_string())
                            .collect(),
                    ),
                };

                group_vec.push(group);
            }
        }

        let target = Target {
            definition: session_value
                .pointer("/target/definition")
                .map(|value| value.as_str().unwrap().to_string()),
            groups: Some(group_vec),
        };

        let mut artifact_vec = Vec::new();

        if let Some(artifact_value_vec) = session_value.pointer("/status/artifacts") {
            for artifact_value in artifact_value_vec.as_array().unwrap() {
                let artifact = Artifact {
                    image_id: artifact_value
                        .get("image_id")
                        .map(|value| value.as_str().unwrap().to_string()),
                    result_id: artifact_value
                        .get("result_id")
                        .map(|value| value.as_str().unwrap().to_string()),
                    r#type: artifact_value
                        .get("type")
                        .map(|value| value.as_str().unwrap().to_string()),
                };
                artifact_vec.push(artifact);
            }
        }

        let session = Session {
            job: session_value
                .pointer("/status/session/job")
                .map(|value| value.as_str().unwrap_or("").to_string()),
            completion_time: session_value
                .pointer("/status/session/completionTime")
                .map(|value| value.as_str().unwrap_or("").to_string()),
            start_time: session_value
                .pointer("/status/session/startTime")
                .map(|value| value.as_str().unwrap_or("").to_string()),
            status: session_value
                .pointer("/status/session/status")
                .map(|value| value.as_str().unwrap_or("").to_string()),
            succeeded: session_value
                .pointer("/status/session/succeeded")
                .map(|value| value.as_str().unwrap_or("").to_string()),
        };

        let status = Status {
            artifacts: Some(artifact_vec),
            session: Some(session),
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

        let session = GetResponse {
            name: session_value["name"].as_str().map(|str| str.to_string()),
            configuration: Some(configuration),
            ansible: Some(ansible),
            target: Some(target),
            status: Some(status),
            tags: Some(tag_vec),
        };

        session
    }
}
