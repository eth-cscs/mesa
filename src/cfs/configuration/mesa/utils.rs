use serde_json::Value;

use crate::{
    cfs::{
        self, configuration::mesa::r#struct::cfs_configuration_response::CfsConfigurationResponse,
    },
    hsm,
};

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
    cfs_configuration_name_opt: Option<&String>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    if let Some(cfs_configuration_name) = cfs_configuration_name_opt {
        cfs_configuration_vec
            .retain(|cfs_configuration| cfs_configuration.name.eq(cfs_configuration_name));
    } else {
        let cfs_components: Vec<Value> = if !hsm_group_name_vec.is_empty() {
            let hsm_group_members = hsm::group::shasta::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            // Note: nodes can be configured calling the component APi directly (bypassing BOS
            // session API)
            crate::cfs::component::mesa::http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &hsm_group_members,
            )
            .await
            .unwrap()
        } else {
            Vec::new()
        };

        let desired_config: Vec<&str> = cfs_components
            .iter()
            .map(|cfs_component| cfs_component["desiredConfig"].as_str().unwrap())
            .collect();

        // We need BOS session templates to find an image created by SAT
        let bos_sessiontemplate_value_vec = crate::bos::template::mesa::http_client::get_all(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
        )
        .await
        .unwrap();

        // We need CFS sessions to find images without a BOS session template
        let mut cfs_session_value_vec = cfs::session::mesa::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
            Some(true),
        )
        .await
        .unwrap();

        cfs::session::mesa::utils::filter_by_hsm(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &mut cfs_session_value_vec,
            hsm_group_name_vec,
            None,
        )
        .await;

        let image_id_cfs_configuration_target_from_bos_sessiontemplate: Vec<(
            String,
            String,
            Vec<String>,
        )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
            bos_sessiontemplate_value_vec,
        );

        let image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
            cfs::session::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
                cfs_session_value_vec,
            );

        let image_id_cfs_configuration_target: Vec<&str> = [
            image_id_cfs_configuration_target_from_bos_sessiontemplate
                .iter()
                .map(|(_, config, _)| config.as_str())
                .collect(),
            image_id_cfs_configuration_target_from_cfs_session
                .iter()
                .map(|(_, config, _)| config.as_str())
                .collect(),
            desired_config,
        ]
        .concat();

        cfs_configuration_vec.retain(|cfs_configuration| {
            hsm_group_name_vec
                .iter()
                .any(|hsm_group| cfs_configuration.name.contains(hsm_group))
                || image_id_cfs_configuration_target.contains(&cfs_configuration.name.as_str())
        });

        cfs_configuration_vec.sort_by(|cfs_configuration_1, cfs_configuration_2| {
            cfs_configuration_1
                .last_updated
                .cmp(&cfs_configuration_2.last_updated)
        });

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            *cfs_configuration_vec = cfs_configuration_vec[cfs_configuration_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
                .to_vec();
        }
    }

    cfs_configuration_vec.to_vec()
}
