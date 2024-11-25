use serde_json::Value;

use crate::{
    cfs::configuration::csm::v3::r#struct::{
        cfs_configuration_request::CfsConfigurationRequest,
        cfs_configuration_response::CfsConfigurationResponse,
    },
    error::Error,
};

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
) -> Result<Vec<CfsConfigurationResponse>, Error> {
    log::info!(
        "Get CFS configuration '{}'",
        configuration_name_opt.unwrap_or("all available")
    );

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

    let api_url: String = if let Some(configuration_name) = configuration_name_opt {
        shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_name
    } else {
        shasta_base_url.to_owned() + "/cfs/v2/configurations"
    };

    let response = client
        .get(api_url)
        .query(&[("limit", stupid_limit)])
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        // Make sure we return a vec if user requesting a single value
        if configuration_name_opt.is_some() {
            let payload = response
                .json::<CfsConfigurationResponse>()
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

pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<CfsConfigurationResponse, Error> {
    log::info!("Create CFS configuration '{}'", configuration_name);
    log::debug!("Create CFS configuration request:\n{:#?}", configuration);

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

    log::debug!(
        "CFS configuration request payload:\n{}",
        serde_json::to_string_pretty(&request_payload).unwrap()
    );

    let response = client
        .put(api_url)
        .json(&request_payload)
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
    configuration_id: &str,
) -> Result<(), Error> {
    log::info!("Delete CFS configuration {:?}", configuration_id);

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    let client = if let Ok(sock5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(sock5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_id;

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
