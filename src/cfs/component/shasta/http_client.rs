use std::error::Error;

use serde_json::Value;

use crate::cfs::component::shasta::r#struct::Component;

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

pub async fn get_multiple_components(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    components_ids: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Value>, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

    let response_rslt = client
        .get(api_url)
        .query(&[("ids", components_ids), ("status", status)])
        .bearer_auth(shasta_token)
        .send()
        .await;

    match response_rslt {
        Ok(response) => response.json::<Vec<Value>>().await,
        Err(error) => Err(error),
    }
}

pub async fn patch_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component: Component,
) -> Result<Vec<Value>, reqwest::Error> {
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

    let response_rslt = client
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component)
        .send()
        .await;

    match response_rslt {
        Ok(response) => response.json::<Vec<Value>>().await,
        Err(error) => Err(error),
    }
}

pub async fn patch_component_list(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_list: Vec<Component>,
) -> Result<Vec<Value>, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

    let response_rslt = client
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component_list)
        .send()
        .await;

    match response_rslt {
        Ok(response) => response.json::<Vec<Value>>().await,
        Err(error) => Err(error),
    }
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
