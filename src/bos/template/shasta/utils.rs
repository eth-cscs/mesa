use serde_json::Value;

/// Get BOS session templates. Ref --> https://apidocs.svc.cscs.ch/paas/bos/operation/get_v1_sessiontemplates/
pub async fn filter(
    bos_sessiontemplate_value_vec: &mut Vec<Value>,
    hsm_group_name_vec: &Vec<String>,
    bos_sessiontemplate_name_opt: Option<&String>,
    cfs_configuration_name_vec_opt: Option<Vec<&str>>,
    limit_number_opt: Option<&u8>,
) {
    if !hsm_group_name_vec.is_empty() {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate_value| {
            bos_sessiontemplate_value["boot_sets"]
                .as_object()
                .is_some_and(|boot_set_obj| {
                    boot_set_obj.iter().any(|(_property, boot_set_param)| {
                        boot_set_param["node_groups"]
                            .as_array()
                            .is_some_and(|node_group_vec| {
                                node_group_vec.iter().any(|node_group| {
                                    hsm_group_name_vec
                                        .contains(&node_group.as_str().unwrap().to_string())
                                })
                            })
                    })
                })
        });
    }

    if let Some(cfs_configuration_name_vec) = cfs_configuration_name_vec_opt {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate_value| {
            cfs_configuration_name_vec.contains(
                &bos_sessiontemplate_value
                    .pointer("/cfs/configuration")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
        });
    }

    if let Some(bos_sessiontemplate_name) = bos_sessiontemplate_name_opt {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate| {
            bos_sessiontemplate["name"]
                .as_str()
                .unwrap()
                .eq(bos_sessiontemplate_name)
        });
    }

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client

        *bos_sessiontemplate_value_vec = bos_sessiontemplate_value_vec
            [bos_sessiontemplate_value_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }
}

pub fn check_hsms_or_xnames_belongs_to_bos_sessiontemplate(
    bos_sessiontemplate: &Value,
    hsm_groups_names: Vec<String>,
    xnames: Vec<String>,
) -> bool {
    let boot_set_type = if bos_sessiontemplate.pointer("/boot_sets/uan").is_some() {
        "uan"
    } else {
        "compute"
    };

    let empty_array_value = &serde_json::Value::Array(Vec::new());

    let bos_template_node_list = bos_sessiontemplate
        .pointer(&("/boot_sets/".to_owned() + boot_set_type + "/node_list"))
        .unwrap_or(empty_array_value)
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node.as_str().unwrap().to_string());

    for bos_template_node in bos_template_node_list {
        if xnames.contains(&bos_template_node) {
            return true;
        }
    }

    let bos_template_node_groups = bos_sessiontemplate
        .pointer(&("/boot_sets/".to_owned() + boot_set_type + "/node_list"))
        .unwrap_or(empty_array_value)
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node.as_str().unwrap().to_string());

    for bos_template_node in bos_template_node_groups {
        if hsm_groups_names.contains(&bos_template_node) {
            return true;
        }
    }

    false
}

pub fn get_image_id_from_bos_sessiontemplate_vec(
    bos_sessiontemplate_value_vec: &[Value],
) -> Vec<String> {
    bos_sessiontemplate_value_vec
        .iter()
        .flat_map(|bos_sessiontemplate_value| {
            bos_sessiontemplate_value["boot_sets"]
                .as_object()
                .unwrap()
                .into_iter()
                .map(|(_, boot_set_param_value)| {
                    boot_set_param_value["path"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("s3://boot-images/")
                        .unwrap()
                        .strip_suffix("/manifest.json")
                        .unwrap()
                        .to_string()
                })
        })
        .collect()
}
