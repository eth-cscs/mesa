use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    bos,
    cfs::{
        self, configuration::mesa::r#struct::cfs_configuration_response::CfsConfigurationResponse,
    },
    common::gitea,
    hsm,
};

use super::r#struct::{
    cfs_configuration_request::CfsConfigurationRequest, cfs_configuration_response::ApiError,
};

pub async fn create_from_sat_file(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    gitea_token: &str,
    cray_product_catalog: &BTreeMap<String, String>,
    sat_file_configuration_yaml: &serde_yaml::Value,
    tag: &str,
) -> Result<CfsConfigurationResponse, ApiError> {
    let mut cfs_configuration = CfsConfigurationRequest::from_sat_file_serde_yaml(
        shasta_root_cert,
        gitea_token,
        sat_file_configuration_yaml,
        cray_product_catalog,
    )
    .await;

    // Rename configuration name
    cfs_configuration.name = cfs_configuration.name.replace("__DATE__", tag);

    /* for cfs_configuration_layer in cfs_configuration.layers.iter_mut() {
        log::info!("CFS configuration layer:\n{:#?}", cfs_configuration_layer);

        if let Some(git_tag) = cfs_configuration_layer.tag.as_ref() {
            log::info!("git tag: {}", git_tag);
            let tag_details = gitea::http_client::get_tag_details(
                &cfs_configuration_layer.clone_url,
                &git_tag,
                gitea_token,
                shasta_root_cert,
            )
            .await
            .unwrap();

            log::info!("tag details:\n{:#?}", tag_details);
            let commit_id: Option<String> =
                tag_details["id"].as_str().map(|commit| commit.to_string());

            cfs_configuration_layer.commit = commit_id;
        }
    } */

    create(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_configuration,
    )
    .await
}

pub async fn create(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration: &mut CfsConfigurationRequest,
) -> Result<CfsConfigurationResponse, ApiError> {
    log::debug!("CFS configuration:\n{:#?}", cfs_configuration);

    let cfs_configuration_rslt = cfs::configuration::mesa::http_client::put(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &cfs_configuration,
        &cfs_configuration.name,
    )
    .await;

    match cfs_configuration_rslt {
        Ok(cfs_configuration) => {
            log::info!(
                "CFS configuration '{}' successfully created",
                cfs_configuration.name
            );
            Ok(cfs_configuration)
        }
        Err(error) => Err(ApiError::CsmError(error.to_string())),
    }
}

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
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
    let mut bos_sessiontemplate_vec =
        bos::template::mesa::http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap();

    bos::template::mesa::utils::filter(
        &mut bos_sessiontemplate_vec,
        hsm_group_name_vec,
        &Vec::new(),
        // None,
        None,
    )
    .await;

    // We need CFS sessions to find images without a BOS session template
    let mut cfs_session_vec = cfs::session::mesa::http_client::get(
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
        &mut cfs_session_vec,
        hsm_group_name_vec,
        None,
    )
    .await;

    let image_id_cfs_configuration_target_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_vec,
    );

    let image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
        cfs::session::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(cfs_session_vec);

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

    cfs_configuration_vec.to_vec()
}
