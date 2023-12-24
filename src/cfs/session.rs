pub mod shasta {

    pub mod http_client {

        use crate::cfs::session::mesa::r#struct::CfsSessionRequest;
        use crate::hsm::utils::get_member_vec_from_hsm_name_vec;

        use serde_json::Value;
        use std::collections::HashSet;
        use std::error::Error;

        /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
        pub async fn get_raw(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            session_name_opt: Option<&String>,
            is_succeded: Option<bool>,
        ) -> Result<reqwest::Response, reqwest::Error> {
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url: String = if let Some(session_name) = session_name_opt {
                shasta_base_url.to_owned() + "/cfs/v2/sessions/" + session_name
            } else {
                shasta_base_url.to_owned() + "/cfs/v2/sessions"
            };

            // Add params to request
            let mut request_payload = Vec::new();

            if is_succeded.is_some() {
                request_payload.push(("succeced", is_succeded));
            }

            let network_response_rslt = client
                .get(api_url)
                .query(&request_payload)
                .bearer_auth(shasta_token)
                .send()
                .await;

            match network_response_rslt {
                Ok(http_response) => http_response.error_for_status(),
                Err(network_error) => Err(network_error),
            }
        }

        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            cfs_session_name_opt: Option<&String>,
            is_succeded: Option<bool>,
        ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }

            let api_url: String = if let Some(cfs_session_name) = cfs_session_name_opt {
                shasta_base_url.to_owned() + "/cfs/v2/sessions/" + cfs_session_name
            } else {
                shasta_base_url.to_owned() + "/cfs/v2/sessions"
            };

            let mut request_payload = Vec::new();

            if is_succeded.is_some() {
                request_payload.push(("succeced", is_succeded));
            }

            let resp = client
                .get(api_url)
                .query(&request_payload)
                .bearer_auth(shasta_token)
                .send()
                .await?;

            let json_response: Value = if resp.status().is_success() {
                serde_json::from_str(&resp.text().await?)?
            } else {
                let response = resp.text().await;
                log::error!("{:#?}", response);
                return Err(response?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
            };

            Ok(json_response.as_array().unwrap().clone())
        }

        pub async fn get_all(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
        ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }

            let api_url = shasta_base_url.to_owned() + "/cfs/v2/sessions";

            let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

            let json_response: Value = if resp.status().is_success() {
                serde_json::from_str(&resp.text().await?)?
            } else {
                let response = resp.text().await;
                log::error!("{:#?}", response);
                return Err(response?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
            };

            Ok(json_response.as_array().unwrap().clone())
        }

        /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
        /// Returns list of CFS sessions filtered by HSM group ordered by start time
        pub async fn filter_by_hsm(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            cluster_cfs_sessions: &mut Vec<Value>,
            hsm_group_name_vec: &[String],
            limit_number_opt: Option<&u8>,
        ) {
            let hsm_group_member_vec = get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
            // hsm_group.members.ids
            cluster_cfs_sessions.retain(|cfs_session| {
                cfs_session["target"]["groups"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .any(|group| {
                        hsm_group_name_vec.contains(&group["name"].as_str().unwrap().to_string())
                    })
                    || cfs_session["ansible"]["limit"]
                        .as_str()
                        .unwrap_or("")
                        .split(',')
                        .map(|node| node.trim().to_string())
                        .collect::<HashSet<_>>()
                        .is_subset(&HashSet::from_iter(hsm_group_member_vec.clone()))
            });

            // Sort CFS sessions by start time order ASC
            cluster_cfs_sessions.sort_by(|a, b| {
                a["status"]["session"]["startTime"]
                    .as_str()
                    .unwrap()
                    .cmp(b["status"]["session"]["startTime"].as_str().unwrap())
            });

            if let Some(limit_number) = limit_number_opt {
                // Limiting the number of results to return to client

                *cluster_cfs_sessions = cluster_cfs_sessions[cluster_cfs_sessions
                    .len()
                    .saturating_sub(*limit_number as usize)..]
                    .to_vec();
            }
        }

        /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
        /// Returns list of CFS sessions ordered by start time
        pub async fn filter(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            cluster_cfs_sessions: &mut Vec<Value>,
            hsm_group_name_vec: &[String],
            limit_number_opt: Option<&u8>,
        ) {
            let hsm_group_member_vec = get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
            // hsm_group.members.ids
            cluster_cfs_sessions.retain(|cfs_session| {
                cfs_session["target"]["groups"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .any(|group| {
                        hsm_group_name_vec.contains(&group["name"].as_str().unwrap().to_string())
                    })
                    || cfs_session["ansible"]["limit"]
                        .as_str()
                        .unwrap_or("")
                        .split(',')
                        .map(|node| node.trim().to_string())
                        .collect::<HashSet<_>>()
                        .is_subset(&HashSet::from_iter(hsm_group_member_vec.clone()))
            });

            // Sort CFS sessions by start time order ASC
            cluster_cfs_sessions.sort_by(|a, b| {
                a["status"]["session"]["startTime"]
                    .as_str()
                    .unwrap()
                    .cmp(b["status"]["session"]["startTime"].as_str().unwrap())
            });

            if let Some(limit_number) = limit_number_opt {
                // Limiting the number of results to return to client

                *cluster_cfs_sessions = cluster_cfs_sessions[cluster_cfs_sessions
                    .len()
                    .saturating_sub(*limit_number as usize)..]
                    .to_vec();
            }
        }

        pub async fn delete(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            session_name: &str,
        ) -> Result<(), Box<dyn Error>> {
            log::info!("Deleting CFS session id: {}", session_name);

            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }

            let api_url = shasta_base_url.to_owned() + "/cfs/v2/sessions/" + session_name;

            let resp = client
                .delete(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await?;

            if resp.status().is_success() {
                log::debug!("{:#?}", resp);
                Ok(())
            } else {
                log::debug!("{:#?}", resp);
                Err(resp.json::<Value>().await?["detail"]
                    .as_str()
                    .unwrap()
                    .into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
            }
        }

        pub async fn post_raw(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            session: &CfsSessionRequest,
        ) -> Result<reqwest::Response, reqwest::Error> {
            log::debug!("Session:\n{:#?}", session);

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url = shasta_base_url.to_owned() + "/cfs/v2/sessions";

            let network_response_rslt = client
                .post(api_url)
                // .post(format!("{}{}", shasta_base_url, "/cfs/v2/sessions"))
                .bearer_auth(shasta_token)
                .json(&session)
                .send()
                .await;

            match network_response_rslt {
                Ok(http_response) => http_response.error_for_status(),
                Err(network_error) => Err(network_error),
            }
        }
    }

    pub mod utils {

        use comfy_table::Table;
        use serde_json::Value;

        pub fn get_image_id_cfs_configuration_target_tuple_vec(
            cfs_session_value_vec: Vec<Value>,
        ) -> Vec<(String, String, Vec<String>)> {
            let mut image_id_cfs_configuration_target_from_cfs_session: Vec<(
                String,
                String,
                Vec<String>,
            )> = Vec::new();

            cfs_session_value_vec.iter().for_each(|cfs_session| {
                if let Some(result_id) = cfs_session.pointer("/status/artifacts/0/result_id") {
                    let target: Vec<String> =
                        if let Some(target_groups) = cfs_session.pointer("/target/groups") {
                            target_groups
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|group| group["name"].as_str().unwrap().to_string())
                                .collect()
                        } else if let Some(ansible_limit) = cfs_session.pointer("/ansible/limit") {
                            ansible_limit
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|xname| xname.as_str().unwrap().to_string())
                                .collect()
                        } else {
                            vec![]
                        };

                    image_id_cfs_configuration_target_from_cfs_session.push((
                        result_id.as_str().unwrap().to_string(),
                        cfs_session
                            .pointer("/configuration/name")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                        target,
                    ));
                } else {
                    image_id_cfs_configuration_target_from_cfs_session.push((
                        "".to_string(),
                        "".to_string(),
                        vec![],
                    ));
                }
            });

            image_id_cfs_configuration_target_from_cfs_session
        }

        pub fn print_table(cfs_sessions: Vec<Vec<String>>) {
            let mut table = Table::new();

            table.set_header(vec![
                "Name",
                "Configuration",
                "Target Def",
                "Target",
                "Start",
                "Status",
                "Succeeded",
                "Image ID",
            ]);

            for cfs_session in cfs_sessions {
                table.add_row(cfs_session);
            }

            println!("{table}");
        }

        pub fn get_image_id_from_cfs_session_vec(cfs_session_value_vec: &[Value]) -> Vec<String> {
            cfs_session_value_vec
                .iter()
                .filter(|cfs_session| {
                    cfs_session
                        .pointer("/target/definition")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .eq("image")
                        && cfs_session
                            .pointer("/status/session/succeeded")
                            .unwrap_or(&serde_json::json!("false"))
                            .as_str()
                            .unwrap()
                            .eq("true")
                        && cfs_session
                            .pointer("/status/artifacts/0/result_id")
                            .is_some()
                })
                .map(|cfs_session| {
                    cfs_session
                        .pointer("/status/artifacts/0/result_id")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        }
    }
}

pub mod mesa {

    pub mod r#struct {

        use serde::{Deserialize, Serialize};

        use serde_json::{json, Value};

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

        impl CfsSessionGetResponse {
            pub fn from_csm_api_json(session_value: Value) -> Self {
                let configuration = Configuration {
                    name: session_value
                        .pointer("/configuration/name")
                        .map(|value| value.as_str().unwrap_or("").to_string()),
                    limit: session_value
                        .pointer("/configuration/limit")
                        .map(|value| value.as_str().unwrap_or("").to_string()),
                };

                let ansible = Ansible {
                    config: session_value
                        .pointer("/ansible/config")
                        .map(|value| value.as_str().unwrap_or("").to_string()),
                    limit: session_value
                        .pointer("/ansible/limit")
                        .map(|value| value.as_str().unwrap_or("").to_string()),
                    verbosity: session_value
                        .pointer("/ansible/verbosity")
                        .map(|str| str.as_u64().unwrap()),
                    passthrough: session_value
                        .pointer("/ansible/passthrough")
                        .map(|value| value.as_str().unwrap_or("").to_string()),
                };

                let mut group_vec = Vec::new();

                if let Some(group_vec_value) = session_value.pointer("/target/groups") {
                    for group_value in group_vec_value.as_array().unwrap_or(&Vec::new()) {
                        let group = Group {
                            name: group_value["name"].as_str().unwrap().to_string(),
                            members: group_value["members"]
                                .as_array()
                                .unwrap_or(&Vec::new())
                                .iter()
                                .map(|str| str.to_string())
                                .collect::<Vec<String>>(),
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

                let session = CfsSessionGetResponse {
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

        #[derive(Debug, Serialize, Deserialize, Clone, Default)]
        pub struct CfsSessionPostRequest {
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
        pub struct Group {
            pub name: String,
            pub members: Vec<String>,
        }

        #[derive(Debug, Serialize, Deserialize, Clone, Default)]
        pub struct Target {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub definition: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub groups: Option<Vec<Group>>,
        }

        impl CfsSessionPostRequest {
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

                if let (Some(groups_name), Some(groups_members)) =
                    (groups_name_opt, groups_members_opt)
                {
                    for (group_name, group_members) in groups_name.iter().zip(groups_members.iter())
                    {
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

                cfs_session.target.definition = Some("image".to_string());
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

                let cfs_session = CfsSessionPostRequest::new(
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
                    definition: Some(
                        session_value
                            .pointer("/target/definition")
                            .unwrap_or(&json!(""))
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
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

                let session = CfsSessionPostRequest {
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

        #[derive(Debug, Serialize, Deserialize, Clone)]
        pub struct CfsSessionRequest {
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
            pub ansible_verbosity: Option<u8>,
            #[serde(rename = "ansiblePassthrough")]
            #[serde(skip_serializing_if = "Option::is_none")]
            pub ansible_passthrough: Option<String>,
            #[serde(default)]
            pub target: Target,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub tags: Option<Tag>,
            #[serde(skip_serializing)]
            pub base_image_id: Option<String>,
        }

        impl Default for CfsSessionRequest {
            fn default() -> Self {
                Self {
                    name: String::default(),
                    configuration_name: String::default(),
                    configuration_limit: None,
                    ansible_limit: None,
                    ansible_config: None,
                    ansible_verbosity: None,
                    ansible_passthrough: None,
                    target: Default::default(),
                    tags: None,
                    base_image_id: Some(String::default()),
                }
            }
        }

        impl CfsSessionRequest {
            pub fn new(
                name: String,
                configuration_name: String,
                ansible_limit: Option<String>,
                ansible_verbosity: Option<u8>,
                ansible_passthrough: Option<String>,
                is_target_definition_image: bool,
                groups_name: Option<Vec<String>>,
                base_image_id: Option<String>,
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
                }

                cfs_session
            }

            pub fn from_sat_file_serde_yaml(session_yaml: &serde_yaml::Value) -> Self {
                let groups_name = session_yaml["configuration_group_names"]
                    .as_sequence()
                    .unwrap()
                    .iter()
                    .map(|group_name| group_name.as_str().unwrap().to_string())
                    .collect();

                let cfs_session = CfsSessionRequest::new(
                    session_yaml["name"].as_str().unwrap().to_string(),
                    session_yaml["configuration"].as_str().unwrap().to_string(),
                    None,
                    None,
                    None,
                    true,
                    Some(groups_name),
                    // Some(base_image_id.to_string()),
                    Some(session_yaml["ims"]["id"].as_str().unwrap().to_string()),
                );
                cfs_session
            }
        }
    }

    pub mod http_client {

        use serde_json::Value;

        use super::r#struct::{CfsSessionGetResponse, CfsSessionRequest};

        /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
        /// Returns list of CFS sessions ordered by start time.
        /// This methods filter by either HSM group name or HSM group members or both
        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            session_name_opt: Option<&String>,
            limit_number_opt: Option<&u8>,
            is_succeded_opt: Option<bool>,
        ) -> Result<Vec<CfsSessionGetResponse>, reqwest::Error> {
            let cfs_session_response = crate::cfs::session::shasta::http_client::get_raw(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                session_name_opt,
                is_succeded_opt,
            )
            .await;

            let cfs_session_response_value: Value = match cfs_session_response {
                Ok(cfs_session_value) => cfs_session_value.json().await.unwrap(),
                Err(error) => return Err(error),
            };

            let mut cfs_session_vec = Vec::new();

            if cfs_session_response_value.is_array() {
                for cfs_session_value in cfs_session_response_value.as_array().unwrap() {
                    cfs_session_vec.push(CfsSessionGetResponse::from_csm_api_json(
                        cfs_session_value.clone(),
                    ));
                }
            } else {
                cfs_session_vec.push(CfsSessionGetResponse::from_csm_api_json(
                    cfs_session_response_value,
                ));
            }

            // Sort CFS sessions by start time order ASC
            cfs_session_vec.sort_by(|a, b| {
                a.status
                    .as_ref()
                    .unwrap()
                    .session
                    .as_ref()
                    .unwrap()
                    .start_time
                    .as_ref()
                    .unwrap()
                    .cmp(
                        b.status
                            .as_ref()
                            .unwrap()
                            .session
                            .as_ref()
                            .unwrap()
                            .start_time
                            .as_ref()
                            .unwrap(),
                    )
            });

            if let Some(limit_number) = limit_number_opt {
                // Limiting the number of results to return to client

                cfs_session_vec = cfs_session_vec
                    [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
                    .to_vec();
            }

            Ok(cfs_session_vec)
        }

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            session: &CfsSessionRequest,
        ) -> Result<CfsSessionGetResponse, reqwest::Error> {
            let cfs_session_response = crate::cfs::session::shasta::http_client::post_raw(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                session,
            )
            .await;

            let cfs_session_response_value: Value = match cfs_session_response {
                Ok(cfs_session_value) => cfs_session_value.json().await.unwrap(),
                Err(error) => return Err(error),
            };

            Ok(CfsSessionGetResponse::from_csm_api_json(
                cfs_session_response_value,
            ))
        }
    }

    pub mod utils {
        use std::collections::HashSet;

        use super::r#struct::CfsSessionGetResponse;

        pub async fn filter_by_hsm(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
            hsm_group_name_vec: &Vec<String>,
            limit_number_opt: Option<&u8>,
        ) {
            let node_vec = crate::hsm::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
            // hsm_group.members.ids
            if !hsm_group_name_vec.is_empty() {
                cfs_session_vec.retain(|cfs_session| {
                    cfs_session.target.clone().is_some_and(|target| {
                        target.groups.is_some_and(|groups| {
                            !groups.is_empty()
                                && groups
                                    .iter()
                                    .any(|group| hsm_group_name_vec.contains(&group.name))
                        })
                    }) || cfs_session.ansible.clone().is_some_and(|ansible| {
                        ansible.limit.is_some_and(|limit| {
                            limit
                                .split(',')
                                .map(|node| node.trim().to_string())
                                .collect::<HashSet<_>>()
                                .is_subset(&HashSet::from_iter(node_vec.clone()))
                        })
                    })
                });

                if let Some(limit_number) = limit_number_opt {
                    // Limiting the number of results to return to client
                    *cfs_session_vec = cfs_session_vec
                        [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
                        .to_vec();
                }
            }
        }

        /* pub async fn filter(
            cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
            hsm_group_name_vec_opt: Option<&Vec<String>>,
            node_vec_opt: Option<&Vec<String>>,
            limit_number_opt: Option<&u8>,
        ) {
            // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
            // hsm_group.members.ids
            if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
                cfs_session_vec.retain(|cfs_session| {
                    cfs_session.target.clone().is_some_and(|target| {
                        target.groups.is_some_and(|groups| {
                            !groups.is_empty()
                                && groups
                                    .iter()
                                    .any(|group| hsm_group_name_vec.contains(&group.name))
                        })
                    }) || cfs_session.ansible.clone().is_some_and(|ansible| {
                        ansible.limit.is_some_and(|limit| {
                            limit
                                .split(',')
                                .map(|node| node.trim().to_string())
                                .collect::<HashSet<_>>()
                                .is_subset(&HashSet::from_iter(node_vec.clone()))
                        })
                    })
                });
            }

            /* // Sort CFS sessions by start time order ASC
            cfs_session_vec.sort_by(|cfs_session_1, cfs_session_2| {
                cfs_session_1
                    .status
                    .clone()
                    .unwrap()
                    .session
                    .unwrap()
                    .start_time
                    .unwrap()
                    .cmp(
                        &cfs_session_2
                            .status
                            .clone()
                            .unwrap()
                            .session
                            .unwrap()
                            .start_time
                            .unwrap(),
                    )
            }); */

            if let Some(limit_number) = limit_number_opt {
                // Limiting the number of results to return to client
                *cfs_session_vec = cfs_session_vec
                    [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
                    .to_vec();
            }
        } */
    }
}

#[cfg(test)]
pub mod test {
    use crate::cfs::session::mesa::r#struct::CfsSessionGetResponse;

    #[tokio::test]
    async fn test_cfs_session_serde_json_to_struct_conversion() {
        let cfs_session_value = serde_json::json!({
          "ansible": {
            "config": "cfs-default-ansible-cfg",
            "limit": "x1005c1s2b0n0,x1005c0s3b0n0",
            "passthrough": null,
            "verbosity": 0
          },
          "configuration": {
            "limit": "",
            "name": "clariden-cos-config-2.3.110-96-3"
          },
          "name": "batcher-e5c059a8-20c1-4779-9c0b-a270ff081d63",
          "status": {
            "artifacts": [],
            "session": {
              "completionTime": "2023-10-10T08:46:34",
              "job": "cfs-298b9145-7504-4241-a985-7a2f301cdd9f",
              "startTime": "2023-10-10T08:36:40",
              "status": "complete",
              "succeeded": "true"
            }
          },
          "tags": {
            "bos_session": "d452344f-4aad-4747-bfcb-8d016b5524bc"
          },
          "target": {
            "definition": "dynamic",
            "groups": null
          }
        });

        let cfs_session = CfsSessionGetResponse::from_csm_api_json(cfs_session_value);

        println!("{:#?}", cfs_session);
    }
}
