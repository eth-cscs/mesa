use serde_json::Value;

use super::r#struct::{Image, ImsImageRecord2Update};

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
) -> Result<Vec<Image>, reqwest::Error> {
    log::info!(
        "Get IMS images '{}'",
        image_id_opt.unwrap_or("all available")
    );

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url = if let Some(image_id) = image_id_opt {
        shasta_base_url.to_owned() + "/ims/v3/images/" + image_id
    } else {
        shasta_base_url.to_owned() + "/ims/v3/images"
    };

    let response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    let image_vec: Vec<Image> = match response_rslt {
        Ok(response) => {
            if image_id_opt.is_none() {
                response.json::<Vec<Image>>().await.unwrap()
            } else {
                vec![response.json::<Image>().await.unwrap()]
            }
        }
        Err(error) => return Err(error),
    };

    Ok(image_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Image>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}

/// Register a new image in IMS --> https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/ims.md#post_v2_image
pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    ims_image: &Image,
) -> Result<Value, reqwest::Error> {
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

    client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&ims_image)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

// Delete IMS image using CSM API. First does a "soft delete", then a "permanent deletion"
// soft delete --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_image/
// permanent deletion --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_deleted_image/
pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id: &str,
) -> Result<(), reqwest::Error> {
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

    client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?
        .error_for_status()?;

    // PERMANENT DELETION
    let api_url = shasta_base_url.to_owned() + "/ims/v3/deleted/images/" + image_id;

    client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?
        .error_for_status()
        .map(|_| ())
}

/// update an IMS image record --> https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/ims.md#post_v2_image
pub async fn patch(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    ims_image_id: &String,
    ims_link: &ImsImageRecord2Update,
) -> Result<Value, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/ims/v3/images/" + &ims_image_id;

    client
        .patch(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&ims_link)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await
}
