use serde_json::Value;

use crate::error::Error;

use super::r#struct::NodeSummary;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname: &str,
) -> Result<NodeSummary, Error> {
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

    let api_url = format!(
        "{}/smd/hsm/v2/Inventory/Hardware/Query/{}",
        shasta_base_url, xname
    );

    let response = client
        .get(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error));

        /* Ok(NodeSummary::from_csm_value(
            payload.unwrap().pointer("/Nodes/0").unwrap().clone(),
        )) */

        match payload.unwrap().pointer("/Nodes/0") {
            Some(node_value) => Ok(NodeSummary::from_csm_value(node_value.clone())),
            None => Err(Error::Message(format!(
                "ERROR - json section '/Node' missing in json response API for node '{}'",
                xname
            ))),
        }
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::CsmError(payload))
    }
}

pub async fn get_hw_inventory(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname: &str,
) -> Result<Value, Error> {
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

    let api_url = format!(
        "{}/smd/hsm/v2/Inventory/Hardware/Query/{}",
        shasta_base_url, xname
    );

    let response = client
        .get(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
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
