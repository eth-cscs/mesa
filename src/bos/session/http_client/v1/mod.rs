use serde_json::{json, Value};

use crate::error::Error;

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template_name: &String,
    operation: &str,
) -> core::result::Result<Value, Error> {
    let payload = json!({
        "operation": operation,
        "templateName": bos_template_name,
        // "limit": limit
    });

    log::info!("Create BOS session v1");
    log::debug!("Create BOS session v1 payload:\n{:#?}", payload);

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

    let api_url = format!("{}{}", shasta_base_url, "/bos/v1/session");

    /* client
    .post(api_url)
    .bearer_auth(shasta_token)
    .json(&json!({
        "operation": operation,
        "templateName": bos_template_name,
        "limit": limit
    }))
    .send()
    .await?
    .error_for_status()?
    .json()
    .await */

    let response = client
        .post(api_url)
        .json(&payload)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        response
            .json()
            .await
            .map_err(|error| Error::NetError(error))
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::CsmError(payload))
    }
}
