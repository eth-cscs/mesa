use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cfs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BootSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_ordinal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_ordinal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_parameters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_roles_groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs_provider_passthrough: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Link {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionTemplate {
    #[serde(rename = "templateUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfs_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfs_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_cfs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfs: Option<Cfs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_sets: Option<Vec<BootSet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Vec<Link>>,
}

impl SessionTemplate {
    pub fn from_csm_api_json(sessiontemplate_value: Value) -> Self {
        let cfs = Cfs {
            clone_url: sessiontemplate_value
                .pointer("/cfs/clone_url")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            branch: sessiontemplate_value
                .pointer("/cfs/branch")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            commit: sessiontemplate_value
                .pointer("/cfs/commit")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            playbook: sessiontemplate_value
                .pointer("/cfs/playbook")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            configuration: sessiontemplate_value
                .pointer("/cfs/configuration")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
        };

        let mut boot_set_vec = Vec::new();
        for (boot_set_index, boot_set_value) in
            sessiontemplate_value["boot_sets"].as_object().unwrap()
        {
            let boot_set = BootSet {
                property: Some(boot_set_index.to_string()),
                name: boot_set_value
                    .get("name")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                boot_ordinal: boot_set_value
                    .get("boot_ordinal")
                    .and_then(|value| value.as_u64()),
                shutdown_ordinal: boot_set_value
                    .get("shutdown_ordinal")
                    .and_then(|value| value.as_u64()),
                path: boot_set_value
                    .get("path")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                r#type: boot_set_value
                    .get("type")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                etag: boot_set_value
                    .get("etag")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                kernel_parameters: boot_set_value
                    .get("kernel_parameters")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                network: boot_set_value
                    .get("property_name")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                node_list: boot_set_value.get("node_list").and_then(|value| {
                    value.as_array().map(|array| {
                        array
                            .iter()
                            .map(|value| value.as_str().unwrap().to_string())
                            .collect::<Vec<String>>()
                    })
                }),
                node_roles_groups: boot_set_value.get("node_roles_groups").and_then(|value| {
                    value.as_array().map(|array| {
                        array
                            .iter()
                            .map(|value| value.as_str().unwrap().to_string())
                            .collect::<Vec<String>>()
                    })
                }),
                node_groups: boot_set_value.get("node_groups").and_then(|value| {
                    value.as_array().map(|array| {
                        array
                            .iter()
                            .map(|value| value.as_str().unwrap().to_string())
                            .collect::<Vec<String>>()
                    })
                }),
                rootfs_provider: boot_set_value
                    .get("property_name")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
                rootfs_provider_passthrough: boot_set_value
                    .get("property_name")
                    .and_then(|value| value.as_str().map(|str| str.to_string())),
            };
            boot_set_vec.push(boot_set)
        }

        let link_vec_opt = if let Some(link_value) = sessiontemplate_value.get("links") {
            link_value.as_array().map(|link_value_vec| {
                link_value_vec
                    .iter()
                    .map(|link_value| Link {
                        rel: Some(link_value["rel"].as_str().unwrap().to_string()),
                        href: Some(link_value["href"].as_str().unwrap().to_string()),
                    })
                    .collect()
            })
        } else {
            None
        };

        Self {
            template_url: sessiontemplate_value
                .pointer("/templateUrl")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            name: sessiontemplate_value
                .pointer("/name")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            description: sessiontemplate_value
                .pointer("/description")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            cfs_url: sessiontemplate_value
                .pointer("/cfs_url")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            cfs_branch: sessiontemplate_value
                .pointer("/cfs_branch")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            enable_cfs: sessiontemplate_value
                .pointer("/enable_cfs")
                .and_then(|value| value.as_bool()),
            cfs: Some(cfs),
            partition: sessiontemplate_value
                .pointer("/partition")
                .and_then(|value| value.as_str().map(|str| str.to_string())),
            boot_sets: Some(boot_set_vec),
            link: link_vec_opt,
        }
    }
}

pub mod http_client {
    use serde_json::Value;

    use crate::shasta;

    use super::SessionTemplate;

    pub async fn get_all(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<SessionTemplate>, reqwest::Error> {
        let bos_sessiontemplate_response_value = shasta::bos::template::http_client::get_raw(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
        )
        .await;

        let bos_sessiontemplate_response_value: Value = match bos_sessiontemplate_response_value {
            Ok(bos_sessiontemplate_value) => bos_sessiontemplate_value.json().await.unwrap(),
            Err(error) => return Err(error),
        };

        let mut bos_sessiontemplate_vec = Vec::new();

        if let Some(bos_sessiontemplate_value_vec) = bos_sessiontemplate_response_value.as_array() {
            for bos_sessiontemplate_value in bos_sessiontemplate_value_vec {
                bos_sessiontemplate_vec.push(SessionTemplate::from_csm_api_json(
                    bos_sessiontemplate_value.clone(),
                ));
            }
        } else {
            bos_sessiontemplate_vec.push(SessionTemplate::from_csm_api_json(
                bos_sessiontemplate_response_value,
            ));
        }

        Ok(bos_sessiontemplate_vec)
    }
}

pub mod utils {
    use comfy_table::Table;
    use serde_json::Value;

    use crate::common::node_ops;

    use super::SessionTemplate;

    pub async fn filter(
        bos_sessiontemplate_vec: &mut Vec<SessionTemplate>,
        hsm_group_name_vec: &Vec<String>,
        hsm_member_vec: &Vec<String>,
        bos_sessiontemplate_name_opt: Option<&String>,
        limit_number_opt: Option<&u8>,
    ) -> Vec<SessionTemplate> {
        bos_sessiontemplate_vec.retain(|bos_sessiontemplate| {
            bos_sessiontemplate
                .boot_sets
                .as_ref()
                .unwrap()
                .iter()
                .any(|boot_set| {
                    (boot_set.node_groups.is_some()
                        && !boot_set.node_groups.as_ref().unwrap().is_empty()
                        && boot_set
                            .node_groups
                            .as_ref()
                            .unwrap()
                            .iter()
                            .all(|node_group| hsm_group_name_vec.contains(node_group)))
                        || (boot_set.node_list.is_some()
                            && !boot_set.node_list.as_ref().unwrap().is_empty()
                            && boot_set
                                .node_list
                                .as_ref()
                                .unwrap()
                                .iter()
                                .all(|node| hsm_member_vec.contains(node)))
                })
        });

        if let Some(bos_sessiontemplate_name) = bos_sessiontemplate_name_opt {
            bos_sessiontemplate_vec.retain(|bos_sessiontemplate| {
                bos_sessiontemplate
                    .name
                    .as_ref()
                    .unwrap()
                    .eq(bos_sessiontemplate_name)
            });
        }

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            *bos_sessiontemplate_vec = bos_sessiontemplate_vec[bos_sessiontemplate_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
                .to_vec();
        }

        bos_sessiontemplate_vec.to_vec()
    }

    pub fn get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_value_vec: Vec<Value>,
    ) -> Vec<(String, String, Vec<String>)> {
        let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
            String,
            String,
            Vec<String>,
        )> = Vec::new();

        for bos_sessiontemplate in bos_sessiontemplate_value_vec {
            if None == bos_sessiontemplate.get("cfs") {
                continue;
            }

            let cfs_configuration = bos_sessiontemplate
                .pointer("/cfs/configuration")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();

            for (_, boot_set) in bos_sessiontemplate
                .pointer("/boot_sets")
                .unwrap()
                .as_object()
                .unwrap()
            {
                let path = boot_set["path"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("s3://boot-images/")
                    .unwrap()
                    .strip_suffix("/manifest.json")
                    .unwrap()
                    .to_string();

                let target: Vec<String> = if let Some(node_groups) = boot_set.get("node_groups") {
                    node_groups
                        .as_array()
                        .unwrap()
                        .into_iter()
                        .map(|node_group| node_group.as_str().unwrap().to_string())
                        .collect()
                } else if let Some(node_list) = boot_set.get("node_list") {
                    node_list
                        .as_array()
                        .unwrap()
                        .into_iter()
                        .map(|target_group| target_group.as_str().unwrap().to_string())
                        .collect()
                } else {
                    vec![]
                };

                image_id_cfs_configuration_from_bos_sessiontemplate.push((
                    path.to_string(),
                    cfs_configuration.to_string(),
                    target,
                ));
            }
        }

        image_id_cfs_configuration_from_bos_sessiontemplate
    }

    pub fn print_table_struct(bos_sessiontemplate_vec: Vec<SessionTemplate>) {
        let mut table = Table::new();

        table.set_header(vec![
            "Name",
            "Cfs Configuration",
            "Cfs Enabled",
            "Type",
            "Target",
            "Compute Etag",
            "Compute Path",
        ]);

        for bos_template in bos_sessiontemplate_vec {
            for boot_set in bos_template.boot_sets.unwrap() {
                let target: Vec<String> = if boot_set.node_groups.is_some() {
                    // NOTE: very
                    // important to
                    // define target
                    // variable type to
                    // tell compiler we
                    // want a long live
                    // variable
                    boot_set.node_groups.unwrap()
                } else if boot_set.node_list.is_some() {
                    boot_set.node_list.unwrap()
                } else {
                    Vec::new()
                };

                table.add_row(vec![
                    bos_template.name.as_ref().unwrap(),
                    bos_template
                        .cfs
                        .as_ref()
                        .unwrap()
                        .configuration
                        .as_ref()
                        .unwrap(),
                    &bos_template.enable_cfs.unwrap().to_string(),
                    &boot_set.property.unwrap(),
                    &node_ops::string_vec_to_multi_line_string(Some(&target), 2),
                    &boot_set.etag.unwrap_or("".to_string()),
                    &boot_set.path.unwrap(),
                ]);
            }
        }

        println!("{table}");
    }
}

#[tokio::test]
async fn test_bos_sessiontemplate_serde_json_to_struct_conversion() {
    let bos_sessiontemplate_value = serde_json::json!({
      "boot_sets": {
        "compute": {
          "etag": "44d82a32878a3abbe461c38b071c55bc",
          "kernel_parameters": "ip=dhcp quiet spire_join_token=${SPIRE_JOIN_TOKEN}",
          "node_groups": [
            "muttler"
          ],
          "path": "s3://boot-images/2105dd38-2c8e-48c5-8b3f-ca71367a977e/manifest.json",
          "rootfs_provider": "cpss3",
          "rootfs_provider_passthrough": "dvs:api-gw-service-nmn.local:300:nmn0",
          "type": "s3"
        }
      },
      "cfs": {
        "configuration": "muttler-cos-config-20221012100753"
      },
      "enable_cfs": true,
      "name": "muttler-cos-template-20221012100753"
    });

    let bos_sessiontemplate = SessionTemplate::from_csm_api_json(bos_sessiontemplate_value);

    println!("{:#?}", bos_sessiontemplate);
}
