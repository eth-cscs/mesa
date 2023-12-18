use std::error::Error;

use serde_json::Value;

use crate::cfs::component::shasta::r#struct::Component;

pub async fn get_single_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy.
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components/" + component_id;

    let resp = client
        .get(api_url)
        // .get(format!("{}{}{}", shasta_base_url, "/cfs/v2/components/", component_id))
        .bearer_auth(shasta_token)
        .send()
        .await?
        .text()
        .await?;

    let json_response: Value = serde_json::from_str(&resp)?;

    Ok(json_response)
}

pub async fn get_multiple_components(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    components_ids: Option<&str>,
    status: Option<&str>,
    // enabled: Option<bool>,
    /* cfs_configuration_name: Option<&str>,
    cfs_configuration_details: Option<bool>, */
    // tags: Option<&str>,
) -> Result<Vec<Value>, Box<dyn Error>> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

    let resp = client
        .get(api_url)
        .query(&[("ids", components_ids), ("status", status)])
        // .get(format!("{}{}{}", shasta_base_url, "/cfs/v2/components/", component_id))
        .bearer_auth(shasta_token)
        .send()
        .await?;

    if resp.status().is_success() {
        let response = &resp.text().await?;
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

pub async fn patch_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component: Component,
) -> Result<Vec<Value>, Box<dyn Error>> {
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

    let api_url =
        shasta_base_url.to_owned() + "/cfs/v2/components/" + &component.clone().id.unwrap();

    let resp = client
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component)
        .send()
        .await?;

    if resp.status().is_success() {
        let response = &resp.text().await?;
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

pub async fn patch_component_list(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_list: Vec<Component>,
) -> Result<Vec<Value>, Box<dyn Error>> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

    let resp = client
        .patch(api_url)
        .bearer_auth(shasta_token)
        .json(&component_list)
        .send()
        .await?;

    if resp.status().is_success() {
        let response = &resp.text().await?;
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

pub async fn delete_single_component(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    component_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy.
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components/" + component_id;

    let resp = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?
        .text()
        .await?;

    let json_response: Value = serde_json::from_str(&resp)?;

    Ok(json_response)
}
