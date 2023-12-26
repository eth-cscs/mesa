use std::error::Error;

use serde_json::Value;

use crate::cfs;

use super::r#struct::Image;

pub async fn get_all_struct(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Image>, Box<dyn Error>> {
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

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    if resp.status().is_success() {
        Ok(resp.json::<Vec<Image>>().await?)
    } else {
        Err(resp.text().await?.into())
    }
}

pub async fn get_all_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Value>, Box<dyn Error>> {
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

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    let mut json_response: Value = if resp.status().is_success() {
        serde_json::from_str(&resp.text().await?)?
    } else {
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    let image_value_vec: Vec<Value> = json_response
        .as_array_mut()
        .unwrap_or(&mut Vec::new())
        .to_vec();

    Ok(image_value_vec.to_vec())
}

pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
    image_name_opt: Option<&str>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut image_vec: Vec<Value> = get_all_raw(shasta_token, shasta_base_url, shasta_root_cert)
        .await
        .unwrap();

    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| {
        a["created"]
            .as_str()
            .unwrap()
            .cmp(b["created"].as_str().unwrap())
    });

    // Limiting the number of results to return to client
    if let Some(limit_number) = limit_number_opt {
        image_vec = image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
    }

    if let Some(image_id) = image_id_opt {
        image_vec.retain(|image_value| image_value["id"].as_str().unwrap().eq(image_id));
    }

    if let Some(image_name) = image_name_opt {
        image_vec.retain(|image_value| image_value["name"].as_str().unwrap().eq(image_name));
    }

    Ok(image_vec.to_vec())
}

/// Fetch IMS image ref --> https://apidocs.svc.cscs.ch/paas/ims/operation/get_v3_image/
/// If filtering by HSM group, then image name must include HSM group name (It assumms each image
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get_struct(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
    image_name_opt: Option<&str>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Box<dyn Error>> {
    let mut image_vec: Vec<Image> = get_all_struct(shasta_token, shasta_base_url, shasta_root_cert)
        .await
        .unwrap();

    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| a.created.as_ref().unwrap().cmp(b.created.as_ref().unwrap()));

    // Limiting the number of results to return to client
    if let Some(limit_number) = limit_number_opt {
        image_vec = image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
    }

    if let Some(image_id) = image_id_opt {
        image_vec.retain(|image_value| image_value.id.as_ref().unwrap().eq(image_id));
    }

    if let Some(image_name) = image_name_opt {
        image_vec.retain(|image_value| image_value.name.eq(image_name));
    }

    Ok(image_vec)
}

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
    limit_number: Option<&u8>,
) -> Vec<(Image, String, String)> {
    let image_vec: Vec<Image> = crate::ims::image::http_client::get_struct(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        limit_number,
    )
    .await
    .unwrap();

    // We need BOS session templates to find an image created by SAT
    let bos_sessiontemplate_value_vec = crate::bos::template::shasta::http_client::get_and_filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    /* println!(
        "DEBUG - BOS sessiontemplate:\n{:#?}",
        bos_sessiontemplates_value_vec
    ); */

    // We need CFS sessions to find images without a BOS session template (hopefully the CFS
    // session has not been deleted by CSCS staff, otherwise it will be technically impossible to
    // find unless we search images by HSM name and expect HSM name to be in image name...)
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
        hsm_group_name_vec,
        None,
    )
    .await;
    /* let cfs_session_value_vec = crate::cfs::session::shasta::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap(); */

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

    /* println!(
        "DEBUG - bos sessiontemplate paths: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        cfs::session::shasta::utils::get_image_id_cfs_configuration_target_tuple_vec(
            cfs_session_value_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    /* println!(
        "DEBUG - cfs sessions: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    // let image_id_from_bos_sessiontemplate_vec = bos_sessiontemplates_value_vec

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

/// Returns a tuple like (Image struct, cfs configuration, target groups) with the cfs
/// configuration related to that image struct and the target groups booting that image
pub async fn get_image_cfsconfiguration_targetgroups_tuple(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &Vec<String>,
    limit_number: Option<&u8>,
) -> Vec<(Image, String, String)> {
    let image_vec: Vec<Image> = crate::ims::image::http_client::get_struct(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        limit_number,
    )
    .await
    .unwrap();

    // We need BOS session templates to find an image created by SAT
    let bos_sessiontemplate_value_vec = crate::bos::template::shasta::http_client::get_and_filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    /* println!(
        "DEBUG - BOS sessiontemplate:\n{:#?}",
        bos_sessiontemplates_value_vec
    ); */

    // We need CFS sessions to find images without a BOS session template
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
        hsm_group_name_vec,
        None,
    )
    .await;
    /* let cfs_session_value_vec = crate::cfs::session::shasta::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap(); */

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

    /* println!(
        "DEBUG - bos sessiontemplate paths: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        cfs::session::shasta::utils::get_image_id_cfs_configuration_target_tuple_vec(
            cfs_session_value_vec,
        );

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    /* println!(
        "DEBUG - cfs sessions: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    // let image_id_from_bos_sessiontemplate_vec = bos_sessiontemplates_value_vec

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

// Delete IMS image using CSM API. First does a "soft delete", then a "permanent deletion"
// soft delete --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_image/
// permanent deletion --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_deleted_image/
pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id: &str,
) -> Result<(), Box<dyn Error>> {
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

    // SOFT DELETION
    let api_url = shasta_base_url.to_owned() + "/ims/v3/images/" + image_id;

    let resp = client
        .delete(api_url)
        // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
        .bearer_auth(shasta_token)
        .send()
        .await?;

    if resp.status().is_success() {
        log::debug!("{:#?}", resp);
    } else {
        log::debug!("{:#?}", resp);
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    }

    // PERMANENT DELETION
    let api_url = shasta_base_url.to_owned() + "/ims/v3/deleted/images/" + image_id;

    let resp = client
        .delete(api_url)
        // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
        .bearer_auth(shasta_token)
        .send()
        .await?;

    if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        Ok(())
    } else {
        log::debug!("{:#?}", resp);
        Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}
