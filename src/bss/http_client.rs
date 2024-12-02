use serde_json::Value;
use tokio::sync::Semaphore;

use core::result::Result;
use std::{sync::Arc, time::Instant};

use crate::error::Error;

use super::r#struct::BootParameters;

pub fn post(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    boot_parameters: BootParameters,
) -> Result<(), Error> {
    let client_builder = reqwest::blocking::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

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

    let api_url = format!("{}/boot/v1/bootparameters", base_url);

    let response = client
        .post(api_url)
        .bearer_auth(auth_token)
        .json(&boot_parameters)
        .send()
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(Error::Message(response.text()?))
    }
}

/// Change nodes boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/put/
pub async fn put(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    boot_parameters: BootParameters,
) -> Result<Vec<Value>, Error> {
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

    let api_url = format!("{}/bss/boot/v1/bootparameters", shasta_base_url);

    log::debug!(
        "request payload:\n{}",
        serde_json::to_string_pretty(&boot_parameters).unwrap()
    );

    let response = client
        .put(api_url)
        .json(&boot_parameters)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        Err(Error::Message(response.text().await?))
    }
}

pub async fn patch(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
    boot_parameters: &BootParameters,
) -> Result<Vec<Value>, Error> {
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

    let api_url = format!("{}/bss/boot/v1/bootparameters", shasta_base_url);

    let response = client
        .patch(api_url)
        .json(&boot_parameters)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        Err(Error::Message(response.text().await?))
    }
}

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xnames: &[String],
) -> Result<Vec<BootParameters>, Error> {
    let start = Instant::now();

    let chunk_size = 30;

    let mut boot_params_vec = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    let sem = Arc::new(Semaphore::new(10)); // CSM 1.3.1 higher number of concurrent tasks won't

    for sub_node_list in xnames.chunks(chunk_size) {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        // let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

        let permit = Arc::clone(&sem).acquire_owned().await;

        let node_vec = sub_node_list.to_vec();

        tasks.spawn(async move {
            let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

            get_raw(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                // &hsm_subgroup_nodes_string,
                &node_vec,
            )
            .await
            .unwrap()
        });
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(mut node_status_vec) = message {
            boot_params_vec.append(&mut node_status_vec);
        }
    }

    let duration = start.elapsed();
    log::info!("Time elapsed to get BSS bootparameters is: {:?}", duration);

    Ok(boot_params_vec)
}

/// Get node boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/get/
pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xnames: &[String],
) -> Result<Vec<BootParameters>, Error> {
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

    let url_api = format!("{}/bss/boot/v1/bootparameters", shasta_base_url.to_string());

    let params: Vec<_> = xnames.iter().map(|xname| ("name", xname)).collect();

    let response = client
        .get(url_api)
        .query(&params)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        response
            .json::<Vec<BootParameters>>()
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
