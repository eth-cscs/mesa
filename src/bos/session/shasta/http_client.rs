use std::error::Error;

use serde_json::{json, Value};

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    id_opt: Option<&str>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let mut api_url = shasta_base_url.to_string() + "/bos/v1/session";

    if let Some(id) = id_opt {
        api_url = api_url + "/" + id
    }

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    let json_response: Value = if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        serde_json::from_str(&resp.text().await?)?
    } else {
        log::debug!("{:#?}", resp);
        // let resp_body = resp.text().await?;
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    // println!("\nBOS SESSIONS:\n{:#?}", json_response);

    Ok(json_response.as_array().unwrap_or(&Vec::new()).to_vec())
}

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template_name: &String,
    operation: &str,
    limit: Option<&String>,
) -> core::result::Result<Value, Box<dyn std::error::Error>> {
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

    let resp = client
        .post(format!("{}{}", shasta_base_url, "/bos/v1/session"))
        .bearer_auth(shasta_token)
        .json(&json!({
            "operation": operation,
            "templateName": bos_template_name,
            "limit": limit
        }))
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(serde_json::from_str(&resp.text().await?)?)
    } else {
        Err(resp.json::<Value>().await?["detail"]
            .as_str()
            .unwrap()
            .into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_id: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_string() + "/bos/v1/session/" + bos_session_id;

    let resp = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?;

    let json_response: Value = if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        serde_json::from_str(&resp.text().await?)?
    } else {
        log::debug!("{:#?}", resp);
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    Ok(json_response)
}
