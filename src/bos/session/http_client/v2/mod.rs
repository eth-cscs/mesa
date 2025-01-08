pub mod types;

use serde_json::Value;
use types::BosSession;

use crate::error::Error;

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session: BosSession,
) -> Result<Value, Error> {
    log::info!("Create BOS session");
    log::debug!("Create BOS session request:\n{:#?}", bos_session);

    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_string() + "/bos/v2/sessions";

    let response = client
        .post(api_url)
        .json(&bos_session)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response
            .json()
            .await
            .map_err(|error| Error::NetError(error))?)
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::CsmError(payload))
    }
}

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    id_opt: Option<&str>,
) -> Result<Vec<BosSession>, Error> {
    log::info!("Get BOS sessions '{}'", id_opt.unwrap_or("all available"));

    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let mut api_url = shasta_base_url.to_string() + "/bos/v2/sessions";

    if let Some(id) = id_opt {
        api_url = api_url + "/" + id
    }

    /* client
    .get(api_url)
    .bearer_auth(shasta_token)
    .send()
    .await?
    .error_for_status()?
    .json()
    .await */

    let response = client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        // Make sure we return a vec if user requesting a single value
        if id_opt.is_some() {
            let payload = response
                .json::<BosSession>()
                .await
                .map_err(|error| Error::NetError(error))?;

            Ok(vec![payload])
        } else {
            response
                .json()
                .await
                .map_err(|error| Error::NetError(error))
        }
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::CsmError(payload))
    }
}

pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_id: &str,
) -> Result<(), Error> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_string() + "/bos/v2/sessions/" + bos_session_id;

    /* client
    .delete(api_url)
    .bearer_auth(shasta_token)
    .send()
    .await?
    .error_for_status()?
    .json()
    .await */

    let response = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::CsmError(payload))
    }
}
