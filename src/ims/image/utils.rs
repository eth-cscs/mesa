use std::error::Error;

use serde_json::Value;

use crate::{
    bos, bss::http_client::get_boot_params,
    hsm::group::shasta::utils::get_member_vec_from_hsm_name_vec, ims::image::r#struct::Image,
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
    hsm_group_name_vec: &[String],
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
/// This method tries to filter by HSM group which means it will make use of:
/// CFS sessions to find which image id was created against which HSM group
/// BOS sessiontemplates to find the HSM group related to nodes being rebooted in the past
/// Image ids in boot params for nodes in HSM groups we are looking for (This is needed to not miss
/// images currenly used which name may not have HSM group we are looking for included not CFS
/// session nor BOS sessiontemplate)
/// Image names with HSM group name included (This is a bad practice because this is a free text
/// prone to human errors)
pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_vec: &mut Vec<Image>,
    hsm_group_name_vec: &[String],
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
        &Vec::new(),
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

    let mut image_id_cfs_configuration_from_cfs_session: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::mesa::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            cfs_session_value_vec.clone(),
        );

    image_id_cfs_configuration_from_cfs_session
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::mesa::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            cfs_session_value_vec,
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
        hsm_group_name_vec,
    )
    .await;

    let boot_param_value_vec = get_boot_params(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &hsm_member_vec,
    )
    .await
    .unwrap_or(Vec::new());

    let image_id_from_boot_params: Vec<String> = boot_param_value_vec
        .iter()
        .map(|boot_param_value| {
            boot_param_value["kernel"]
                .as_str()
                .unwrap()
                .strip_prefix("s3://boot-images/")
                .unwrap()
                .strip_suffix("/kernel")
                .unwrap()
                .to_string()
        })
        .collect();

    // Get Image details from IMS images API endpoint
    let mut image_detail_vec: Vec<(Image, String, String)> = Vec::new();

    for image in image_vec {
        let image_id = image.id.as_ref().unwrap();

        let target_group_name_vec: Vec<String>;
        let cfs_configuration: String;

        if let Some(tuple) = image_id_cfs_configuration_from_cfs_session
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            // Image details in CFS session
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            // Image details in BOS session template
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if image_id_from_boot_params.contains(image_id) {
            // Image details where image is found in a node boot param related to HSM we are
            // working with
            cfs_configuration = "Not found".to_string();
            target_group_name_vec = vec![];
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            // Image details where the image name contains the HSM group name we are filtering (This
            // is a bad practice hence image name is a free text and user may make mistakes typing
            // it but CSCS staff deletes the CFS sessions therefore we should do this to fetch as
            // much related images as we can)
            cfs_configuration = "Not found".to_string();
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
    hsm_group_name_vec: &[String],
    limit_number_opt: Option<&u8>,
) -> Vec<(Image, String, String)> {
    let mut image_vec: Vec<Image> =
        super::mesa::http_client::get(shasta_token, shasta_base_url, shasta_root_cert, None)
            .await
            .unwrap();

    super::mesa::utils::filter(&mut image_vec).await;

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
        &Vec::new(),
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
        crate::cfs::session::mesa::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
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

/// Register a new image in IMS --> https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/ims.md#post_v2_image
pub async fn register_new_image(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    ims_image: &Image,
) -> Result<Value, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/ims/v3/images";

    let resp = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&ims_image)
        .send()
        .await?;

    let json_response: Value;

    if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        json_response = serde_json::from_str(&resp.text().await?)?;
        Ok(json_response)
    } else {
        log::debug!("{:#?}", resp);
        Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}
