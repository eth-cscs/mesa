use crate::{
    bos,
    bss::http_client::get_multiple,
    error::Error,
    hsm::group::utils::get_member_vec_from_hsm_name_vec,
    ims::{self, image::http_client::types::Image},
};

// Get Image using fuzzy finder, meaning returns any image which name contains a specific
// string.
// Used to find an image created through a CFS session and has not been renamed because manta
// does not rename the images as SAT tool does for the sake of keeping the original image ID in
// the CFS session which created the image.
pub async fn get_fuzzy(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_name_available_vec: &[String],
    image_name_opt: Option<&str>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
    let mut image_available_vec: Vec<Image> = get_image_available_vec(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_name_available_vec,
        None, // NOTE: don't put any limit here since we may be looking in a large number of
              // HSM groups and we will filter the results by image name below
    )
    .await?;

    if let Some(image_name) = image_name_opt {
        image_available_vec.retain(|image| image.name.contains(image_name));
    }

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        image_available_vec = image_available_vec[image_available_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }

    Ok(image_available_vec.to_vec())
}

pub async fn get_by_name(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_name_available_vec: &[String],
    image_name_opt: Option<&str>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
    let mut image_available_vec: Vec<Image> = get_image_available_vec(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_name_available_vec,
        None, // NOTE: don't put any limit here since we may be looking in a large number of
              // HSM groups and we will filter the results by image name below
    )
    .await?;

    if let Some(image_name) = image_name_opt {
        image_available_vec.retain(|image| image.name.eq(image_name));
    }

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        image_available_vec = image_available_vec[image_available_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }

    Ok(image_available_vec.to_vec())
}

/// Just sorts images by creation time in ascendent order
pub async fn filter(image_vec: &mut [Image]) {
    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| a.created.as_ref().unwrap().cmp(b.created.as_ref().unwrap()));
}

/// Returns a tuple like(Image sruct, cfs configuration name, list of target - either hsm group name
/// or xnames, bool - indicates if image is used to boot a node or not)
/// This method tries to filter by HSM group which means it will make use of:
///  - CFS sessions to find which image id was created against which HSM group
///  - BOS sessiontemplates to find the HSM group related to nodes being rebooted in the past
///  - Image ids in boot params for nodes in HSM groups we are looking for (This is needed to not miss
/// images currenly used which name may not have HSM group we are looking for included not CFS
/// session nor BOS sessiontemplate)
///  - Image names with HSM group name included (This is a bad practice because this is a free text
/// prone to human errors)
pub async fn get_image_cfs_config_name_hsm_group_name(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_vec: &mut Vec<Image>,
    hsm_group_name_vec: &[String],
    limit_number_opt: Option<&u8>,
) -> Result<Vec<(Image, String, String, bool)>, Error> {
    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        *image_vec = image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
    }

    // We need BOS session templates to find an image created by SAT
    let mut bos_sessiontemplate_value_vec = crate::bos::template::http_client::v2::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
    )
    .await?;

    bos::template::utils::filter(
        &mut bos_sessiontemplate_value_vec,
        hsm_group_name_vec,
        &Vec::new(),
        // None,
        None,
    );

    // We need CFS sessions to find images without a BOS session template (hopefully the CFS
    // session has not been deleted by CSCS staff, otherwise it will be technically impossible to
    // find unless we search images by HSM name and expect HSM name to be in image name...)
    let mut cfs_session_vec = crate::cfs::session::get_and_sort(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        None,
        Some(true),
    )
    .await?;

    crate::cfs::session::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_vec,
        hsm_group_name_vec,
        None,
        true,
    )
    .await?;

    let mut image_id_cfs_configuration_from_cfs_session: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            cfs_session_vec.clone(),
        );

    image_id_cfs_configuration_from_cfs_session
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            cfs_session_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    // Get IMAGES in nodes boot params. This is because CSCS staff deletes the CFS sessions and/or
    // BOS sessiontemplate breaking the history with actual state, therefore I need to go to boot
    // params to get the image id used to boot the nodes belonging to a HSM group
    let hsm_member_vec = get_member_vec_from_hsm_name_vec(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec.to_vec(),
    )
    .await?;

    let boot_param_vec = get_multiple(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &hsm_member_vec,
    )
    .await
    .unwrap_or_default();

    let image_id_from_boot_params: Vec<String> = boot_param_vec
        .iter()
        .map(|boot_param| boot_param.get_boot_image())
        .collect();

    // Get Image details from IMS images API endpoint
    let mut image_detail_vec: Vec<(Image, String, String, bool)> = Vec::new();

    for image in image_vec {
        let image_id = image.id.as_ref().unwrap();

        let target_group_name_vec: Vec<String>;
        let cfs_configuration: String;
        let target_groups: String;
        let boot_image: bool;

        if let Some(tuple) = image_id_cfs_configuration_from_cfs_session
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            // Image details in CFS session
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
            target_groups = target_group_name_vec.join(", ");
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            // Image details in BOS session template
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
            target_groups = target_group_name_vec.join(", ");
        } else if let Some(boot_params) = boot_param_vec
            .iter()
            .find(|boot_params| boot_params.get_boot_image().eq(image_id))
        {
            // Image details where image is found in a node boot param related to HSM we are
            // working with
            // Boot params don't have CFS configuration information
            cfs_configuration = "Not found".to_string();
            target_groups = boot_params.hosts.clone().join(",");
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            // Image details where image name contains HSM group name available to the user.
            // Boot params don't have CFS configuration information
            // NOTE: CSCS specific
            cfs_configuration = "Not found".to_string();

            target_groups = "Not found".to_string();
        } else {
            continue;
        }

        // NOTE: 'boot_image' needs to be processed outside the 'if' statement. Otherwise we may
        // miss images used to boot nodes filtered by a different branch in the 'if' statement
        boot_image = if image_id_from_boot_params.contains(image_id) {
            true
        } else {
            false
        };

        image_detail_vec.push((
            image.clone(),
            cfs_configuration.to_string(),
            target_groups.clone(),
            boot_image,
        ));
    }

    Ok(image_detail_vec)
}

/// Returns a list of images with the cfs
/// configuration related to that image struct and the target groups booting that image
/// This list is filtered by the HSM groups the user has access to
/// Exception are images containing 'generic' in their names since those could be used by anyone
pub async fn get_image_available_vec(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_name_available_vec: &[String],
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
    let mut image_vec: Vec<Image> =
        super::http_client::get(shasta_token, shasta_base_url, shasta_root_cert, None).await?;

    ims::image::utils::filter(&mut image_vec).await;

    // We need BOS session templates to find an image created by SAT
    let mut bos_sessiontemplate_vec =
        bos::template::http_client::v2::get(shasta_token, shasta_base_url, shasta_root_cert, None)
            .await?;

    // Filter BOS sessiontemplates to the ones the user has access to
    bos::template::utils::filter(
        &mut bos_sessiontemplate_vec,
        hsm_name_available_vec,
        &Vec::new(),
        None,
    );

    // We need CFS sessions to find images without a BOS session template
    let mut cfs_session_vec = crate::cfs::session::get_and_sort(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        None,
        Some(true),
    )
    .await?;

    // Filter CFS sessions to the ones the user has access to
    crate::cfs::session::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_vec,
        hsm_name_available_vec,
        None,
        true,
    )
    .await?;

    let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_vec,
    );

    image_id_cfs_configuration_from_bos_sessiontemplate
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            cfs_session_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    let mut image_available_vec: Vec<Image> = Vec::new();

    for image in &image_vec {
        let image_id = image.id.as_ref().unwrap();

        if image_id_cfs_configuration_from_bos_sessiontemplate
            .iter()
            .any(|tuple| tuple.0.eq(image_id))
        {
            // If image is related to a BOS sessiontemplate related to a HSM group the user has
            // access to, then, we include this image to the list of images available to the user
            image_available_vec.push(image.clone());
        } else if image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .any(|tuple| tuple.0.eq(image_id))
        {
            // If image was created using a CFS session with HSM groups related to the user, then
            // we include this image to the list of images available to the user
            // FIXME: this needs to go away if we extend groups in CFS sessions to technology
            // rather than clusters
            image_available_vec.push(image.clone());
        } else if hsm_name_available_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            // If image name contains HSM group the user is working on, then, we include the image
            // to the list of images available to the user
            // FIXME: this should not be allowed... but CSCS staff deletes the CFS sessions so we
            // are extending the rules that defines if a user has access to an image
            image_available_vec.push(image.clone());
        } else if image.name.to_lowercase().contains("generic") {
            // If image is generic (meaning image name contains the word "generic"), then, the image
            // will be available to everyone, therefore it should be included to the list of images
            // available to the user
            // FIXME: This is should not be allowed since it is too vague, we concept of generic is
            // not limited to anything, a tenant may create an image which name contains "generic"
            // but they don't want to share it with other tenants meaning the scope of generic here
            // does not moves across tenants boundaries
            image_available_vec.push(image.clone())
        } else {
            continue;
        }

        // let target_groups = target_group_name_vec.join(", ");
    }

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        image_available_vec = image_available_vec[image_available_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }

    Ok(image_available_vec)
}
