use serde_json::Value;

use crate::error::Error;

use super::r#struct::{PowerCapComponent, PowerCapTaskInfo};

pub async fn get(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
) -> Result<PowerCapTaskInfo, Error> {
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

    let api_url = format!("{}/power-control/v1/power-cap", shasta_base_url);

    let response = client
        .get(api_url)
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

pub async fn get_task_id(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    task_id: &str,
) -> Result<PowerCapTaskInfo, Error> {
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

    let api_url = format!("{}/power-control/v1/power-cap/{}", shasta_base_url, task_id);

    let response = client
        .get(api_url)
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

pub async fn post_snapshot(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    xname_vec: Vec<&str>,
) -> Result<PowerCapTaskInfo, Error> {
    log::info!("Create PCS power snapshot for nodes:\n{:?}", xname_vec);
    log::debug!("Create PCS power snapshot for nodes:\n{:?}", xname_vec);

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

    let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-cap/snapshot";

    let response = client
        .put(api_url)
        .json(&serde_json::json!({
            "xnames": xname_vec
        }))
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

pub async fn patch(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    power_cap: Vec<PowerCapComponent>,
) -> Result<PowerCapTaskInfo, Error> {
    log::info!("Create PCS power cap:\n{:#?}", power_cap);
    log::debug!("Create PCS power cap:\n{:#?}", power_cap);

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

    let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-cap/snapshot";

    let response = client
        .put(api_url)
        .json(&power_cap)
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
