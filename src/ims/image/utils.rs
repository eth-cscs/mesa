use std::error::Error;

use crate::{bos, ims::image::r#struct::Image};

// Get Image using fuzzy finder, meaning returns any image which name contains a specific
// string.
// Used to find an image created through a CFS session and has not been renamed because manta
// does not rename the images as SAT tool does for the sake of keeping the original image ID in
// the CFS session which created the image.
pub async fn get_fuzzy(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &Vec<String>,
    image_name_opt: Option<&str>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<(Image, String, String)>, Box<dyn Error>> {
    let mut image_configuration_hsm_group_tuple_vec: Vec<(Image, String, String)> =
        get_image_cfsconfiguration_targetgroups_tuple(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name_vec,
            limit_number_opt,
        )
        .await;

    if let Some(image_name) = image_name_opt {
        image_configuration_hsm_group_tuple_vec
            .retain(|(image, _, _)| image.name.contains(image_name));
    }

    Ok(image_configuration_hsm_group_tuple_vec.to_vec())
}

/// Returns a tuple like(Image sruct, cfs configuration name, list of target - either hsm group name
/// or xnames)
pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_vec: &mut Vec<Image>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<(Image, String, String)> {
    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        *image_vec = image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
    }

    // We need BOS session templates to find an image created by SAT
    let mut bos_sessiontemplate_value_vec = crate::bos::template::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
    )
    .await
    .unwrap();

    bos::template::mesa::utils::filter(
        &mut bos_sessiontemplate_value_vec,
        hsm_group_name_vec,
        None,
        None,
    )
    .await;

    // We need CFS sessions to find images without a BOS session template (hopefully the CFS
    // session has not been deleted by CSCS staff, otherwise it will be technically impossible to
    // find unless we search images by HSM name and expect HSM name to be in image name...)
    let mut cfs_session_value_vec = crate::cfs::session::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        Some(true),
    )
    .await
    .unwrap();

    crate::cfs::session::mesa::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_value_vec,
        hsm_group_name_vec,
        None,
    )
    .await;

    // println!("DEBUG - CFS session:\n{:#?}", cfs_session_vec);
    let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_value_vec,
    );

    image_id_cfs_configuration_from_bos_sessiontemplate
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
            cfs_session_value_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    let mut image_detail_vec: Vec<(Image, String, String)> = Vec::new();

    for image in image_vec {
        let image_id = image.id.as_ref().unwrap();

        let target_group_name_vec: Vec<String>;
        let cfs_configuration: String;

        if let Some(tuple) = image_id_cfs_configuration_from_bos_sessiontemplate
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            cfs_configuration = "".to_string();
            target_group_name_vec = vec![];
        } else {
            continue;
        }

        let target_groups = target_group_name_vec.join(", ");

        image_detail_vec.push((
            image.clone(),
            cfs_configuration.to_string(),
            target_groups.clone(),
        ));
    }

    image_detail_vec
}

/// Returns a tuple like (Image struct, cfs configuration, target groups) with the cfs
/// configuration related to that image struct and the target groups booting that image
pub async fn get_image_cfsconfiguration_targetgroups_tuple(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<(Image, String, String)> {
    let mut image_vec: Vec<Image> =
        super::mesa::http_client::get(shasta_token, shasta_base_url, shasta_root_cert, None)
            .await
            .unwrap();

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        image_vec = image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
    }

    // We need BOS session templates to find an image created by SAT
    let mut bos_sessiontemplate_vec = bos::template::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
    )
    .await
    .unwrap();

    bos::template::mesa::utils::filter(
        &mut bos_sessiontemplate_vec,
        hsm_group_name_vec,
        None,
        None,
    )
    .await;

    // We need CFS sessions to find images without a BOS session template
    let mut cfs_session_vec = crate::cfs::session::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        Some(true),
    )
    .await
    .unwrap();

    crate::cfs::session::mesa::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_vec,
        hsm_group_name_vec,
        None,
    )
    .await;

    let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_vec,
    );

    image_id_cfs_configuration_from_bos_sessiontemplate
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
            cfs_session_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    let mut image_detail_vec: Vec<(Image, String, String)> = Vec::new();

    for image in &image_vec {
        let image_id = image.id.as_ref().unwrap();

        let target_group_name_vec: Vec<String>;
        let cfs_configuration: String;

        if let Some(tuple) = image_id_cfs_configuration_from_bos_sessiontemplate
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            cfs_configuration = "".to_string();
            target_group_name_vec = vec![];
        } else {
            continue;
        }

        let target_groups = target_group_name_vec.join(", ");

        image_detail_vec.push((
            image.clone(),
            cfs_configuration.to_string(),
            target_groups.clone(),
        ));
    }

    image_detail_vec
}
