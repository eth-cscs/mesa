use serde_json::Value;

use crate::error::Error;

use super::r#struct::Membership;

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Membership>, Error> {
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

    let api_url = format!("{}/smd/hsm/v2/memberships", shasta_base_url);

    let response = client
        .get(api_url.clone())
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response
            .json::<Vec<Membership>>()
            .await
            .map_err(|error| Error::NetError(error))
            .unwrap())
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}

pub async fn get_xname(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname: &str,
) -> Result<Membership, Error> {
    log::info!("Get membership of node '{}'", xname);
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

    let api_url = format!("{}/smd/hsm/v2/memberships/{}", shasta_base_url, xname);

    let response = client
        .get(api_url.clone())
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response
            .json::<Membership>()
            .await
            .map_err(|error| Error::NetError(error))
            .unwrap())
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}
