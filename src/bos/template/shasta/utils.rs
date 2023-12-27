use comfy_table::Table;
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

pub fn print_table(bos_templates: Vec<Value>) {
    let mut table = Table::new();

    table.set_header(vec![
        "Name",
        "Cfs configuration",
        "Cfs enabled",
        "Compute Node groups",
        "Compute Etag",
        "Compute Path",
        "UAN Node groups",
        "UAN Etag",
        "UAN Path",
    ]);

    for bos_template in bos_templates {
        let mut compute_target_groups = String::new();
        let mut uan_target_groups;

        if bos_template["boot_sets"].get("uan").is_some() {
            let uan_node_groups_json = bos_template["boot_sets"]["uan"]["node_groups"].as_array();

            if let Some(uan_node_groups_json_aux) = uan_node_groups_json {
                uan_target_groups = String::from(uan_node_groups_json_aux[0].as_str().unwrap());
            } else {
                uan_target_groups = "".to_string();
            }

            for (i, _) in uan_node_groups_json.iter().enumerate().skip(1) {
                if i % 2 == 0 {
                    // breaking the cell content into multiple lines (only 2 target groups per line)
                    uan_target_groups.push_str(",\n");
                    // uan_target_groups = format!("{},\n", uan_target_groups);
                } else {
                    uan_target_groups.push_str(", ");
                    // uan_target_groups = format!("{}, ", uan_target_groups);
                }

                uan_target_groups.push_str(uan_node_groups_json.unwrap()[i].as_str().unwrap());

                // uan_target_groups = format!("{}{}", uan_target_groups, uan_node_groups_json[i].as_str().unwrap());
            }
        }

        if bos_template["boot_sets"].get("compute").is_some() {
            let compute_node_groups_json =
                bos_template["boot_sets"]["compute"]["node_groups"].as_array();

            if let Some(compute_node_groups_json_aux) = compute_node_groups_json {
                compute_target_groups =
                    String::from(compute_node_groups_json_aux[0].as_str().unwrap());
            } else {
                compute_target_groups = "".to_string();
            }

            for (i, _) in compute_node_groups_json.iter().enumerate().skip(1) {
                if i % 2 == 0 {
                    // breaking the cell content into multiple lines (only 2 target groups per line)

                    compute_target_groups.push_str(",\n");

                    // compute_target_groups = format!("{},\n", compute_target_groups);
                } else {
                    compute_target_groups.push_str(", ");

                    // compute_target_groups = format!("{}, ", compute_target_groups);
                }

                compute_target_groups
                    .push_str(compute_node_groups_json.unwrap()[i].as_str().unwrap());

                // compute_target_groups = format!("{}{}", compute_target_groups, compute_node_groups_json[i].as_str().unwrap());
            }
        }

        table.add_row(vec![
            bos_template["name"].as_str().unwrap(),
            bos_template["cfs"]["configuration"].as_str().unwrap(),
            &bos_template["enable_cfs"].as_bool().unwrap().to_string(),
            &compute_target_groups,
            bos_template["boot_sets"]["compute"]["etag"]
                .as_str()
                .unwrap_or_default(),
            bos_template["boot_sets"]["compute"]["path"]
                .as_str()
                .unwrap_or_default(),
            bos_template["boot_sets"]["uan"]["node_groups"]
                .as_str()
                .unwrap_or_default(),
            bos_template["boot_sets"]["uan"]["etag"]
                .as_str()
                .unwrap_or_default(),
            bos_template["boot_sets"]["uan"]["path"]
                .as_str()
                .unwrap_or_default(),
        ]);
    }

    println!("{table}");
}
