use serde_json::Value;

pub fn get_hsm_group_from_cfs_session_related_to_cfs_configuration(
    cfs_session_value_vec: &Vec<Value>,
    cfs_configuration: &str,
) -> Vec<String> {
    let mut hsm_group_from_cfs_session_vec = cfs_session_value_vec
        .iter()
        .filter(|cfs_session| {
            cfs_session
                .pointer("/configuration/name")
                .unwrap()
                .eq(cfs_configuration)
        })
        .flat_map(|cfs_session| {
            cfs_session
                .pointer("/target/groups")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|group| group["name"].as_str().unwrap().to_string())
        })
        .collect::<Vec<String>>();

    hsm_group_from_cfs_session_vec.sort();
    hsm_group_from_cfs_session_vec.dedup();

    hsm_group_from_cfs_session_vec
}

pub fn get_hsm_group_from_bos_sessiontimplate_related_to_cfs_configuration(
    bos_sessiontemplate_value_vec: &Vec<Value>,
    cfs_configuration: &str,
) -> Vec<String> {
    let hsm_group_from_bos_sessiontemplate_computer_related_to_cfs_configuration =
        bos_sessiontemplate_value_vec
            .iter()
            .filter(|bos_sessiontemplate| {
                bos_sessiontemplate
                    .pointer("/cfs/configuration")
                    .unwrap()
                    .eq(cfs_configuration)
            })
            .flat_map(|bos_sessiontemplate| {
                bos_sessiontemplate
                    .pointer("/boot_sets/compute/node_groups")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|node_group| node_group.as_str().unwrap().to_string())
            });

    let hsm_group_from_bos_sessiontemplate_uan_related_to_cfs_configuration =
        bos_sessiontemplate_value_vec
            .iter()
            .filter(|bos_sessiontemplate| {
                bos_sessiontemplate
                    .pointer("/cfs/configuration")
                    .unwrap()
                    .eq(cfs_configuration)
                    && bos_sessiontemplate
                        .pointer("/boot_sets/uan/node_groups")
                        .is_some()
            })
            .flat_map(|bos_sessiontemplate| {
                bos_sessiontemplate
                    .pointer("/boot_sets/uan/node_groups")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|node_group| node_group.as_str().unwrap().to_string())
            });

    let mut hsm_group_from_bos_sessiontemplate_vec =
        hsm_group_from_bos_sessiontemplate_computer_related_to_cfs_configuration
            .chain(hsm_group_from_bos_sessiontemplate_uan_related_to_cfs_configuration)
            .collect::<Vec<String>>();

    hsm_group_from_bos_sessiontemplate_vec.sort();
    hsm_group_from_bos_sessiontemplate_vec.dedup();

    hsm_group_from_bos_sessiontemplate_vec
}
