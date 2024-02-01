use serde_json::Value;

use crate::hsm;

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_value_vec: &mut Vec<Value>,
    hsm_group_name_vec_opt: Option<&Vec<String>>,
    most_recent_opt: Option<bool>,
    limit_number_opt: Option<&u8>,
) {
    // FILTER BY HSM GROUP NAMES
    if !hsm_group_name_vec_opt.unwrap().is_empty() {
        if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
            let hsm_group_member_vec = hsm::group::shasta::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            let mut cfs_session_vec = crate::cfs::session::mesa::http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                None,
                None,
            )
            .await
            .unwrap();

            crate::cfs::session::mesa::utils::filter_by_hsm(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &mut cfs_session_vec,
                hsm_group_name_vec_opt.unwrap(),
                limit_number_opt,
            )
            .await;

            let cfs_configuration_name_vec_from_cfs_session = cfs_session_vec
                .iter()
                .map(|cfs_session| cfs_session.configuration.clone().unwrap().name.unwrap())
                .collect::<Vec<_>>();

            let bos_sessiontemplate_vec = crate::bos::template::mesa::http_client::get_all(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
            )
            .await
            .unwrap()
            .into_iter()
            .filter(|bos_sessiontemplate| {
                /* let boot_set_vec = bos_sessiontemplate
                    .clone()
                    .boot_sets
                    .clone()
                    .unwrap_or_default();

                let mut boot_set_node_groups_vec =
                    boot_set_vec.iter().flat_map(|(_parameter, boot_set)| {
                        boot_set.clone().node_groups.clone().unwrap_or_default()
                    });

                let mut boot_set_node_list_vec =
                    boot_set_vec.iter().flat_map(|(_parameter, boot_set)| {
                        boot_set.clone().node_list.clone().unwrap_or_default()
                    }); */

                let boot_set_node_groups_vec = bos_sessiontemplate.get_target_hsm();
                let boot_set_node_list_vec = bos_sessiontemplate.get_target_xname();

                boot_set_node_groups_vec.len() > 0
                    && boot_set_node_groups_vec
                        .iter()
                        .all(|node_group| hsm_group_name_vec.contains(&node_group))
                    || boot_set_node_list_vec.len() > 0
                        && boot_set_node_list_vec
                            .iter()
                            .all(|xname| hsm_group_member_vec.contains(&xname))
            })
            .collect::<Vec<_>>();

            let cfs_configuration_name_from_bos_sessiontemplate = bos_sessiontemplate_vec
                .iter()
                .map(|bos_sessiontemplate| {
                    bos_sessiontemplate.get_confguration().unwrap()
                    /* bos_sessiontemplate
                    .cfs
                    .clone()
                    .unwrap()
                    .configuration
                    .clone()
                    .unwrap() */
                })
                .collect::<Vec<_>>();

            let cfs_configuration_name_from_cfs_session_and_bos_settiontemplate = [
                cfs_configuration_name_vec_from_cfs_session,
                cfs_configuration_name_from_bos_sessiontemplate,
            ]
            .concat();

            configuration_value_vec.retain(|cfs_configuration| {
                cfs_configuration_name_from_cfs_session_and_bos_settiontemplate
                    .contains(&cfs_configuration["name"].as_str().unwrap().to_string())
            });
        }
    }

    configuration_value_vec.sort_by(|a, b| {
        a["lastUpdated"]
            .as_str()
            .unwrap()
            .cmp(b["lastUpdated"].as_str().unwrap())
    });

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        *configuration_value_vec = configuration_value_vec[configuration_value_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }

    if most_recent_opt.is_some() && most_recent_opt.unwrap() {
        *configuration_value_vec = [configuration_value_vec.first().unwrap().clone()].to_vec();
    }
}
