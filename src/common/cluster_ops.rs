use serde_json::Value;

use crate::hsm::http_client::get_hsm_group_vec;

#[derive(Debug)]
pub struct ClusterDetails {
    pub hsm_group_label: String,
    pub most_recent_cfs_configuration_name_created: Value,
    pub most_recent_cfs_session_name_created: Value,
    pub members: Vec<Value>,
}

pub async fn get_details(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &str,
) -> Vec<ClusterDetails> {
    let mut clusters_details = vec![];

    // Get HSM groups matching cluster name
    let hsm_group_value_vec = get_hsm_group_vec(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        Some(&hsm_group_name.to_string()),
    )
    .await
    .unwrap();

    for hsm_group in hsm_group_value_vec {
        let hsm_group_name = hsm_group["label"].as_str().unwrap();

        let hsm_group_members: String =
            crate::hsm::utils::get_member_vec_from_hsm_group_value(&hsm_group).join(",");

        // Get all CFS sessions
        let mut cfs_session_value_vec = crate::cfs::session::shasta::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
            Some(true),
        )
        .await
        .unwrap();

        crate::cfs::session::shasta::http_client::filter(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &mut cfs_session_value_vec,
            &[hsm_group_name.to_string()],
            None,
        )
        .await;
        /* let cfs_session_value_vec = crate::cfs::session::shasta::http_client::filter(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &[hsm_group_name.to_string()],
            None,
            None,
            None,
        )
        .await
        .unwrap(); */

        let most_recent_cfs_session;
        let cfs_configuration;

        for cfs_session_value in cfs_session_value_vec {
            // println!("cfs_session_value:\n{:#?}", cfs_session_value);
            let target_groups_option = cfs_session_value.pointer("/target/groups");
            let target_groups = if let Some(Value::Array(target_group_vec)) = target_groups_option {
                target_group_vec.clone()
            } else {
                Vec::new()
            };
            let ansible_limit_option = cfs_session_value.pointer("/ansible/limit");
            let ansible_limit = if let Some(ansible_limit) = ansible_limit_option {
                ansible_limit.as_str().unwrap().to_string()
            } else {
                "".to_string()
            };

            // Check CFS session is linkged to HSM GROUP name or any of its members
            if target_groups
                .iter()
                .map(|target_group| target_group["name"].as_str().unwrap())
                .collect::<Vec<&str>>()
                .contains(&hsm_group_name)
                || ansible_limit.contains(&hsm_group_members)
            {
                most_recent_cfs_session = cfs_session_value;

                // Get CFS configuration linked to CFS session related to HSM GROUP or any of its
                // members
                let cfs_configuration_value_vec =
                    crate::cfs::configuration::shasta::http_client::get(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        Some(
                            &most_recent_cfs_session
                                .pointer("/configuration/name")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_string(),
                        ),
                    )
                    .await
                    .unwrap();

                cfs_configuration = cfs_configuration_value_vec.first().unwrap();

                let cluster_details = ClusterDetails {
                    hsm_group_label: hsm_group_name.to_string(),
                    most_recent_cfs_configuration_name_created: cfs_configuration.clone(),
                    most_recent_cfs_session_name_created: most_recent_cfs_session,
                    members: hsm_group["members"]["ids"].as_array().unwrap().clone(),
                };

                clusters_details.push(cluster_details);

                break;
            }
        }
    }

    clusters_details
}
