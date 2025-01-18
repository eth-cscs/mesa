pub mod types;

use std::{sync::Arc, time::Instant};

use serde_json::Value;
use tokio::sync::Semaphore;
use types::ComponentVec;

use crate::{cfs::component::http_client::v3::types::Component, error::Error};

pub async fn get_options(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v3/options";

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
        response
            .json::<ComponentVec>()
            .await
            .map(|component_vec| component_vec.components)
            .map_err(|e| Error::NetError(e))
    } else {
        let payload = response
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}

pub async fn get_single_by_id(
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

/// Get components data.
/// Currently, CSM will throw an error if many xnames are sent in the request, therefore, this
/// method will paralelize multiple calls, each with a batch of xnames
pub async fn get_parallel(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    node_vec: &[String],
) -> Result<Vec<Component>, Error> {
    let start = Instant::now();

    let num_xnames_per_request = 60;
    let pipe_size = 15;

    log::debug!(
        "Number of nodes per request: {num_xnames_per_request}; Pipe size (semaphore): {pipe_size}"
    );

    let mut component_vec = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    let sem = Arc::new(Semaphore::new(pipe_size)); // CSM 1.3.1 higher number of concurrent tasks won't

    let num_requests = (node_vec.len() / num_xnames_per_request) + 1;

    let mut i = 1;

    // Calculate number of digits of a number (used for pretty formatting console messages)
    let width = num_requests.checked_ilog10().unwrap_or(0) as usize + 1;

    for sub_node_list in node_vec.chunks(num_xnames_per_request) {
        let num_nodes_in_flight = sub_node_list.len();
        log::info!(
            "Getting CFS components: processing batch [{i:>width$}/{num_requests}] (batch size - {num_nodes_in_flight})"
        );

        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

        let permit = sem.clone().acquire_owned().await.unwrap();

        tasks.spawn(async move {
            let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

            get_query(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                Some(&hsm_subgroup_nodes_string),
                None,
            )
            .await
            .unwrap()
        });

        i += 1;
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(mut cfs_component_vec) = message {
            component_vec.append(&mut cfs_component_vec);
        }
    }

    let duration = start.elapsed();
    log::info!("Time elapsed to get CFS components is: {:?}", duration);

    Ok(component_vec)
}

pub async fn get_query(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name: Option<&str>,
    components_ids: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Component>, Error> {
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

    let response = client
        .get(api_url)
        .query(&[
            ("ids", components_ids),
            ("config_name", configuration_name),
            ("status", status),
            ("limit", Some(&stupid_limit.to_string())),
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
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}

pub async fn patch_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component: Component,
) -> Result<Vec<Value>, Error> {
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
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component)
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

pub async fn patch_component_list(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_list: Vec<Component>,
) -> Result<Vec<Value>, Error> {
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
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component_list)
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
