use serde_json::Value;

use crate::error::Error;

use super::r#struct::{CfsSessionGetResponse, CfsSessionPostRequest};

/// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    min_age_opt: Option<&String>,
    max_age_opt: Option<&String>,
    status_opt: Option<&String>,
    session_name_opt: Option<&String>,
    is_succeded_opt: Option<bool>,
) -> Result<Vec<CfsSessionGetResponse>, Error> {
    log::info!(
        "Get CFS sessions '{}'",
        session_name_opt.unwrap_or(&"all available".to_string())
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

    let api_url: String = if let Some(session_name) = session_name_opt {
        shasta_base_url.to_owned() + "/cfs/v2/sessions/" + session_name
    } else {
        shasta_base_url.to_owned() + "/cfs/v2/sessions"
    };

    // Add params to request
    let mut request_payload = Vec::new();

    if let Some(is_succeded) = is_succeded_opt {
        request_payload.push(("succeced", is_succeded.to_string()));
    }

    if let Some(min_age) = min_age_opt {
        request_payload.push(("min_age", min_age.to_string()));
    }

    if let Some(max_age) = max_age_opt {
        request_payload.push(("max_age", max_age.to_string()));
    }

    if let Some(status) = status_opt {
        request_payload.push(("status", status.to_string()));
    }

    let response = client
        .get(api_url)
        .query(&request_payload)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        // Make sure we return a vec if user requesting a single value
        if session_name_opt.is_some() {
            let payload = response
                .json::<CfsSessionGetResponse>()
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

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    session: &CfsSessionPostRequest,
) -> Result<CfsSessionGetResponse, Error> {
    log::debug!("Session:\n{:#?}", session);

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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/sessions";

    let response = client
        .post(api_url)
        .json(&session)
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

pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    session_name: &str,
) -> Result<(), Error> {
    log::info!("Deleting CFS session id: {}", session_name);

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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/sessions/" + session_name;

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
