pub mod r#struct;

use serde_json::Value;

use crate::{cfs::component::http_client::v2::r#struct::ComponentRequest, error::Error};

pub async fn get_single_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_id: &str,
) -> Result<Value, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components/" + component_id;

    let response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match response_rslt {
        Ok(response) => response.json().await,
        Err(error) => Err(error),
    }
}

pub async fn put_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component: ComponentRequest,
) -> Result<Value, Error> {
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

    let api_url =
        shasta_base_url.to_owned() + "/cfs/v2/components/" + &component.clone().id.unwrap();

    let response = client
        .put(api_url)
        .bearer_auth(shasta_token)
        .json(&component)
        .send()
        .await
        .map_err(|e| Error::NetError(e))?;

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

pub async fn put_component_list(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_list: Vec<ComponentRequest>,
) -> Vec<Result<Value, Error>> {
    let mut result_vec = Vec::new();

    for component in component_list {
        let result =
            put_component(shasta_token, shasta_base_url, shasta_root_cert, component).await;
        result_vec.push(result);
    }

    result_vec
}

pub async fn delete_single_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_id: &str,
) -> Result<Value, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components/" + component_id;

    let response_rslt = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await;

    match response_rslt {
        Ok(response) => response.json().await,
        Err(error) => Err(error),
    }
}
