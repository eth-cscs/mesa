use comfy_table::Table;
use serde_json::Value;

use crate::{
    bos::template::mesa::r#struct::response_payload::BosSessionTemplate, common::node_ops,
};

// use super::response_payload::BosSessionTemplate;

pub async fn filter(
    bos_sessiontemplate_vec: &mut Vec<BosSessionTemplate>,
    hsm_group_name_vec: &[String],
    hsm_member_vec: &[String],
    bos_sessiontemplate_name_opt: Option<&String>,
    limit_number_opt: Option<&u8>,
    cfs_configuration_name_opt: Option<&str>,
) -> Vec<BosSessionTemplate> {
    // Filter by target (hsm group name or xnames)
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

    if let Some(cfs_configuration_name) = cfs_configuration_name_opt {
        bos_sessiontemplate_vec.retain(|bos_sessiontemplate| {
            bos_sessiontemplate.cfs.as_ref().is_some_and(|cfs| {
                cfs.configuration
                    .as_ref()
                    .is_some_and(|configuration| configuration.eq(cfs_configuration_name))
            })
        })
    }

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
        if bos_sessiontemplate.get("cfs").is_none() {
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
                    .iter()
                    .map(|node_group| node_group.as_str().unwrap().to_string())
                    .collect()
            } else if let Some(node_list) = boot_set.get("node_list") {
                node_list
                    .as_array()
                    .unwrap()
                    .iter()
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

pub fn print_table_struct(bos_sessiontemplate_vec: Vec<BosSessionTemplate>) {
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
