pub mod r#struct;

use r#struct::ComponentVec;
use serde_json::Value;

use crate::{cfs::component::http_client::v3::r#struct::Component, error::Error};

pub async fn get_options(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/options";

    client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?
        .json()
        .await
}

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    components_ids: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Component>, Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/components";

    let response = client
        .get(api_url)
        .query(&[("ids", components_ids), ("status", status)])
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response.json::<ComponentVec>().await?.components)
    } else {
        let payload = response
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}

pub async fn get_single_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_id: &str,
) -> Result<Component, Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/components/" + component_id;

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
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}

pub async fn get_multiple_components(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    components_ids: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Component>, reqwest::Error> {
    let stupid_limit = 100000;

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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/components";

    let response_rslt = client
        .get(api_url)
        .query(&[
            ("ids", components_ids),
            ("status", status),
            ("limit", Some(&stupid_limit.to_string())),
        ])
        .bearer_auth(shasta_token)
        .send()
        .await;

    match response_rslt {
        Ok(response) => {
            let component_vec = &response.json::<ComponentVec>().await?.components;
            Ok(component_vec.to_vec())
        }
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
        shasta_base_url.to_owned() + "/cfs/v3/components/" + &component.clone().id.unwrap();

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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/components";

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

pub async fn put_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component: Component,
) -> Result<Component, Error> {
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
        shasta_base_url.to_owned() + "/cfs/v3/components/" + &component.clone().id.unwrap();

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
            .json()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}

pub async fn put_component_list(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_list: Vec<Component>,
) -> Vec<Result<Component, Error>> {
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
) -> Result<Component, Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/components/" + component_id;

    let response = client
        .delete(api_url)
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
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}
