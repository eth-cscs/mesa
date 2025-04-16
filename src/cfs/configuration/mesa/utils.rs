use crate::{
    bos::{self, template::mesa::r#struct::v2::BosSessionTemplate},
    cfs::{
        self, component::shasta::r#struct::v2::ComponentResponse,
        session::mesa::r#struct::v2::CfsSessionGetResponse,
    },
    common, hsm,
    ims::image::r#struct::Image,
};

use globset::Glob;

use super::r#struct::{
    cfs_configuration_request::v2::CfsConfigurationRequest,
    cfs_configuration_response::v2::CfsConfigurationResponse,
};

pub async fn create(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration_name: &str,
    cfs_configuration: &CfsConfigurationRequest,
) -> Result<CfsConfigurationResponse, crate::error::Error> {
    cfs::configuration::mesa::http_client::put(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        cfs_configuration,
        cfs_configuration_name,
    )
    .await
}

/// Filter the list of CFS configurations provided. This operation is very expensive since it is
/// filtering by HSM group which means it needs to link CFS configurations with CFS sessions and
/// BOS sessiontemplate. Aditionally, it will also fetch CFS components to find CFS sessions and
/// BOS sessiontemplates linked to specific xnames that also belongs to the HSM group the user is
/// filtering from.
pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
    configuration_name_pattern_opt: Option<&str>,
    hsm_group_name_vec: &[String],
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    log::info!("Filter CFS configurations");
    // Fetch CFS components and filter by HSM group members
    let cfs_component_vec: Vec<ComponentResponse> = if !hsm_group_name_vec.is_empty() {
        let hsm_group_members = hsm::group::utils::get_member_vec_from_hsm_name_vec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name_vec.to_vec(),
        )
        .await;

        // Note: nodes can be configured calling the component APi directly (bypassing BOS
        // session API)
        crate::cfs::component::mesa::http_client::get_multiple(
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

    /* // Fetch CFS sessions
    let mut cfs_session_vec = cfs::session::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap();

    // Fetch BOS sessiontemplates
    let mut bos_sessiontemplate_vec =
        bos::template::mesa::http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap(); */

    let (_, cfs_session_vec_opt, bos_sessiontemplate_vec_opt, _) =
        crate::common::utils::get_configurations_sessions_bos_sessiontemplates_images(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            false,
            true,
            true,
            false,
        )
        .await;

    let mut cfs_session_vec = cfs_session_vec_opt.unwrap();
    let mut bos_sessiontemplate_vec = bos_sessiontemplate_vec_opt.unwrap();

    // Filter BOS sessiontemplates based on HSM groups
    bos::template::mesa::utils::filter(
        &mut bos_sessiontemplate_vec,
        hsm_group_name_vec,
        &Vec::new(),
        // None,
        None,
    )
    .await;

    // Filter CFS sessions based on HSM groups
    cfs::session::mesa::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_vec,
        hsm_group_name_vec,
        None,
        true,
    )
    .await;

    // Get boot image id and desired configuration from BOS sessiontemplates
    let image_id_cfs_configuration_target_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_vec,
    );

    // Get image id, configuration and targets from CFS sessions
    let image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
        cfs::session::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(cfs_session_vec);

    // Get desired configuration from CFS components
    let desired_config_vec: Vec<String> = cfs_component_vec
        .into_iter()
        .map(|cfs_component| cfs_component.desired_config.unwrap())
        .collect();

    // Merge CFS configurations in list of filtered CFS sessions and BOS sessiontemplates and
    // desired configurations in CFS components
    let cfs_configuration_in_cfs_session_and_bos_sessiontemplate: Vec<String> = [
        image_id_cfs_configuration_target_from_bos_sessiontemplate
            .into_iter()
            .map(|(_, config, _)| config)
            .collect(),
        image_id_cfs_configuration_target_from_cfs_session
            .into_iter()
            .map(|(_, config, _)| config)
            .collect(),
        desired_config_vec,
    ]
    .concat();

    // Filter CFS configurations
    cfs_configuration_vec.retain(|cfs_configuration| {
        hsm_group_name_vec
            .iter()
            .any(|hsm_group| cfs_configuration.name.contains(hsm_group))
            || cfs_configuration_in_cfs_session_and_bos_sessiontemplate
                .contains(&cfs_configuration.name)
    });

    // Sort by last updated date in ASC order
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

    // Filter CFS configurations based on mattern matching
    if let Some(configuration_name_pattern) = configuration_name_pattern_opt {
        let glob = Glob::new(configuration_name_pattern)
            .unwrap()
            .compile_matcher();
        cfs_configuration_vec
            .retain(|cfs_configuration| glob.is_match(cfs_configuration.name.clone()));
    }

    cfs_configuration_vec.to_vec()
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get_and_filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name: Option<&str>,
    configuration_name_pattern: Option<&str>,
    hsm_group_name_vec: &[String],
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    let mut cfs_configuration_value_vec: Vec<CfsConfigurationResponse> =
        crate::cfs::configuration::mesa::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            configuration_name,
        )
        .await
        .unwrap_or_default();

    let is_user_admin_rslt = common::jwt_ops::is_user_admin(shasta_token);

    if let Ok(true) = is_user_admin_rslt {
        // Do nothing
    } else {
        // If user is not admin or function checking if user is admin fails, then filter CFS
        // configurations
        crate::cfs::configuration::mesa::utils::filter(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &mut cfs_configuration_value_vec,
            configuration_name_pattern,
            hsm_group_name_vec,
            limit_number_opt,
        )
        .await;
    }

    cfs_configuration_value_vec
}

// Get all CFS sessions, IMS images and BOS sessiontemplates related to a CFS configuration
pub async fn get_derivatives(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name: &str,
) -> (
    Option<Vec<CfsSessionGetResponse>>,
    Option<Vec<BosSessionTemplate>>,
    Option<Vec<Image>>,
) {
    // List of image ids from CFS sessions and BOS sessiontemplates related to CFS configuration
    let mut image_id_vec: Vec<String> = Vec::new();

    /* // Get CFS sessions related to CFS configuration
    //
    let mut cfs_sessions = cfs::session::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Get BOS sessiontemplate related to CFS configuration
    //
    let mut bos_sessiontemplates =
        bos::template::mesa::http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap();

    // Get Images from CFS sessions
    //
    let mut images = crate::ims::image::mesa::http_client::get_all(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
    )
    .await
    .unwrap(); */

    let (_, cfs_sessions_opt, bos_sessiontemplates_opt, ims_images_opt) =
        crate::common::utils::get_configurations_sessions_bos_sessiontemplates_images(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            false,
            true,
            true,
            true,
        )
        .await;

    let mut cfs_sessions = cfs_sessions_opt.unwrap();
    let mut bos_sessiontemplates = bos_sessiontemplates_opt.unwrap();
    let mut ims_images = ims_images_opt.unwrap();

    // Filter CFS sessions
    cfs::session::mesa::utils::filter_by_cofiguration(&mut cfs_sessions, configuration_name);

    // Filter BOS sessiontemplate
    bos_sessiontemplates.retain(|bos_sessiontemplate| {
        bos_sessiontemplate
            .get_image_vec()
            .iter()
            .any(|image_id_aux| image_id_vec.contains(image_id_aux))
            || bos_sessiontemplate.get_confguration().unwrap_or_default() == configuration_name
    });

    // Add all image ids in CFS sessions into image_id_vec
    image_id_vec.extend(
        cfs_sessions
            .iter()
            .flat_map(|cfs_session| cfs_session.get_result_id_vec().into_iter()),
    );

    // Add boot images from BOS sessiontemplate to image_id_vec
    image_id_vec.extend(
        bos_sessiontemplates
            .iter()
            .flat_map(|bos_sessiontemplate| bos_sessiontemplate.get_image_vec()),
    );

    // Filter images
    ims_images.retain(|image| image_id_vec.contains(image.id.as_ref().unwrap()));

    (
        Some(cfs_sessions),
        Some(bos_sessiontemplates),
        Some(ims_images),
    )
}
