pub mod types;

use serde_json::Value;

use crate::{
    cfs::session::http_client::v3::types::{
        CfsSessionGetResponse, CfsSessionGetResponseList, CfsSessionPostRequest,
    },
    error::Error,
};

/// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    session_name_opt: Option<&String>,
    limit_opt: Option<u8>,
    after_id_opt: Option<String>,
    min_age_opt: Option<String>,
    max_age_opt: Option<String>,
    status_opt: Option<String>,
    name_contains_opt: Option<String>,
    is_succeded_opt: Option<bool>,
    tags_opt: Option<String>,
) -> Result<Vec<CfsSessionGetResponse>, Error> {
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
        shasta_base_url.to_owned() + "/cfs/v3/sessions/" + session_name
    } else {
        shasta_base_url.to_owned() + "/cfs/v3/sessions"
    };

    // Add params to request
    let mut request_payload = Vec::new();

    if let Some(limit) = limit_opt {
        request_payload.push(("limit", limit.to_string()));
    }

    if let Some(after_id) = after_id_opt {
        request_payload.push(("after_id", after_id.to_string()));
    }

    if let Some(min_age) = min_age_opt {
        request_payload.push(("min_age", min_age));
    }

    if let Some(max_age) = max_age_opt {
        request_payload.push(("max_age", max_age));
    }

    if let Some(status) = status_opt {
        request_payload.push(("status", status));
    }

    if let Some(name_contains) = name_contains_opt {
        request_payload.push(("name_contains", name_contains));
    }

    if let Some(is_succeded) = is_succeded_opt {
        request_payload.push(("succeced", is_succeded.to_string()));
    }

    if let Some(tags) = tags_opt {
        request_payload.push(("tags", tags));
    }

    let response = client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        // Make sure we return a vec if user requesting a single value
        if session_name_opt.is_some() {
            response
                .json::<CfsSessionGetResponse>()
                .await
                .map(|payload| vec![payload])
                .map_err(|error| Error::NetError(error))
        } else {
            response
                .json::<CfsSessionGetResponseList>()
                .await
                .map(|payload| payload.sessions)
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/sessions";

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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/sessions/" + session_name;

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
