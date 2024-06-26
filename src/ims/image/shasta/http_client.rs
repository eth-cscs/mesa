use serde_json::Value;

pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
) -> Result<reqwest::Response, reqwest::Error> {
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

    match response_rslt {
        Ok(response) => response.error_for_status(),
        Err(error) => Err(error),
    }
}

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
) -> Result<Vec<Value>, reqwest::Error> {
    let resp = get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        image_id_opt,
    )
    .await?
    .error_for_status()?;

    let mut image_value_vec: Vec<Value> = if image_id_opt.is_some() {
        [resp.json::<Value>().await?].to_vec()
    } else {
        resp.json().await?
    };

    // Sort images by creation time order ASC
    image_value_vec.sort_by(|a, b| {
        a["created"]
            .as_str()
            .unwrap()
            .cmp(b["created"].as_str().unwrap())
    });

    Ok(image_value_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Value>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
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
