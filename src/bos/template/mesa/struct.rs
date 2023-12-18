pub mod request_payload {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default)]
    pub struct Link {
        #[serde(skip_serializing_if = "Option::is_none")]
        rel: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, Default)]
    pub struct Property {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub boot_ordinal: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shutdown_ordinal: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(rename = "type")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub type_prop: Option<String>,
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

    #[derive(Debug, Serialize, Deserialize, Default)]
    pub struct BootSet {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub compute: Option<Property>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub uan: Option<Property>,
    }

    #[derive(Debug, Serialize, Deserialize, Default)]
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

    #[derive(Debug, Serialize, Deserialize)]
    pub struct BosSessionTemplate {
        pub name: String,
        #[serde(rename = "templateUrl")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub template_url: Option<String>,
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
        pub boot_sets: Option<BootSet>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub links: Option<Vec<Link>>,
    }

    //////////////////////////////////////////////////////////

    pub struct BootArtifacts {
        pub kernel: Option<String>,
        pub kernel_parameters: Option<String>,
        pub rootfs: Option<String>,
        pub initrd: Option<String>,
    }

    pub struct DesiredState {
        pub boot_artifacts: Option<BootArtifacts>,
        pub configuration: Option<String>,
    }

    pub struct LastAction {
        pub action: Option<String>,
        pub num_attempts: Option<u32>,
    }

    pub struct Component {
        pub id: Option<String>,
        pub actual_state: Option<BootArtifacts>,
        pub desired_state: Option<DesiredState>,
        pub last_action: Option<LastAction>,
        pub enabled: Option<bool>,
        pub error: Option<String>,
    }

    impl BosSessionTemplate {
        /* pub fn from_sat_file_serde_yaml(bos_template_yaml: &serde_yaml::Value) -> Self {

            BosTemplate
        } */

        pub fn new_for_node_list(
            bos_session_template_name: String,
            cfs_configuration_name: Option<String>,
            ims_image_name: Option<String>,
            ims_image_path: Option<String>,
            ims_image_type: Option<String>,
            ims_image_etag: Option<String>,
            limit: Option<Vec<String>>,
        ) -> Self {
            let cfs = Cfs {
                clone_url: None,
                branch: None,
                commit: None,
                playbook: None,
                configuration: cfs_configuration_name,
            };

            let compute_property = Property {
            name: ims_image_name,
            boot_ordinal: Some(2),
            shutdown_ordinal: None,
            path: ims_image_path,
            type_prop: ims_image_type,
            etag: ims_image_etag,
            kernel_parameters: Some(
                "ip=dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.disable_default_svc=0 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN}".to_string(),
            ),
            network: Some("nmn".to_string()),
            node_list: limit,
            node_roles_groups: None,
            node_groups: None,
            rootfs_provider: Some("cpss3".to_string()),
            rootfs_provider_passthrough: Some("dvs:api-gw-service-nmn.local:300:nmn0".to_string()),
        };

            let boot_set = BootSet {
                compute: Some(compute_property),
                uan: None,
            };

            BosSessionTemplate {
                name: bos_session_template_name,
                template_url: None,
                description: None,
                cfs_url: None,
                cfs_branch: None,
                enable_cfs: Some(true),
                cfs: Some(cfs),
                partition: None,
                boot_sets: Some(boot_set),
                links: None,
            }
        }

        pub fn new_for_hsm_group(
            cfs_configuration_name: String,
            bos_session_template_name: String,
            ims_image_name: String,
            ims_image_path: String,
            ims_image_type: String,
            ims_image_etag: String,
            hsm_group: &String,
        ) -> Self {
            let cfs = Cfs {
                clone_url: None,
                branch: None,
                commit: None,
                playbook: None,
                configuration: Some(cfs_configuration_name),
            };

            let compute_property = Property {
            name: Some(ims_image_name),
            boot_ordinal: Some(2),
            shutdown_ordinal: None,
            path: Some(ims_image_path),
            type_prop: Some(ims_image_type),
            etag: Some(ims_image_etag),
            kernel_parameters: Some(
                "ip=dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.disable_default_svc=0 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN}".to_string(),
            ),
            network: Some("nmn".to_string()),
            node_list: None,
            node_roles_groups: None,
            node_groups: Some(vec![hsm_group.to_string()]),
            rootfs_provider: Some("cpss3".to_string()),
            rootfs_provider_passthrough: Some("dvs:api-gw-service-nmn.local:300:nmn0".to_string()),
        };

            let boot_set = BootSet {
                compute: Some(compute_property),
                uan: None,
            };

            BosSessionTemplate {
                name: bos_session_template_name,
                template_url: None,
                description: None,
                cfs_url: None,
                cfs_branch: None,
                enable_cfs: Some(true),
                cfs: Some(cfs),
                partition: None,
                boot_sets: Some(boot_set),
                links: None,
            }
        }
    }
}

pub mod response_payload {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Link {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rel: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub href: Option<String>,
    }

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
    pub struct BosSessionTemplate {
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

    impl BosSessionTemplate {
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
}
