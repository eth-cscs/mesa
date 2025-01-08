use serde_json::Value;

use crate::error::Error;

use super::types::PowerStatus;

pub async fn get(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    xname_vec_opt: Option<&[&str]>,
    power_state_filter_opt: Option<&str>,
    management_state_filter_opt: Option<&str>,
) -> Result<PowerStatus, Error> {
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

    let api_url = format!("{}/power-control/v1/power-status", shasta_base_url);

    let xname_vec_str_opt: Option<String> = xname_vec_opt.map(|xname_vec| xname_vec.join(","));

    let response = client
        .get(api_url)
        .query(&[
            ("xname", xname_vec_str_opt),
            (
                "powerStateFilter",
                power_state_filter_opt.map(|value| value.to_string()),
            ),
            (
                "managementStateFilter",
                management_state_filter_opt.map(|value| value.to_string()),
            ),
        ])
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

pub async fn post(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    power_status: PowerStatus,
) -> Result<PowerStatus, Error> {
    log::info!("Create PCS power status:\n'{:#?}'", power_status);
    log::debug!("Create PCS power status:\n{:#?}", power_status);

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

    let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-status";

    let response = client
        .put(api_url)
        .json(&power_status)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|e| Error::NetError(e))?;

    if response.status().is_success() {
        Ok(response.json().await.map_err(|e| Error::NetError(e))?)
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|e| Error::NetError(e))?;

        Err(Error::CsmError(payload))
    }
}
