use std::error::Error;

use crate::cfs::configuration::mesa::r#struct::cfs_configuration_request::CfsConfigurationRequest;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
) -> Result<reqwest::Response, reqwest::Error> {
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

    let api_url: String = if let Some(configuration_name) = configuration_name_opt {
        shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_name
    } else {
        shasta_base_url.to_owned() + "/cfs/v2/configurations"
    };

    let response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match response_rslt {
        Ok(response) => response.error_for_status(),
        Err(error) => Err(error),
    }
}

pub async fn put_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<reqwest::Response, reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_name;

    let request_payload = serde_json::json!({"layers": configuration.layers});
    log::info!(
        "CFS configuration request payload:\n{}",
        serde_json::to_string_pretty(&request_payload).unwrap()
    );

    client
        .put(api_url)
        .json(&request_payload) // Encapsulating configuration.layers
        .bearer_auth(shasta_token)
        .send()
        .await
}

pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_id: &str,
) -> Result<(), Box<dyn Error>> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_id;

    let resp = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?;

    if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        Ok(())
    } else {
        log::debug!("{:#?}", resp);
        Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}
