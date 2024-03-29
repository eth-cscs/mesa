use serde_json::Value;

use crate::bos::template::mesa::r#struct::request_payload::BosSessionTemplate;

/// Get BOS session templates. Ref --> https://apidocs.svc.cscs.ch/paas/bos/operation/get_v1_sessiontemplates/
pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&String>,
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

    let api_url = if let Some(bos_session_template_id) = bos_session_template_id_opt {
        shasta_base_url.to_owned() + "/bos/v1/sessiontemplate/" + bos_session_template_id
    } else {
        shasta_base_url.to_owned() + "/bos/v1/sessiontemplate"
    };

    let response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match response_rslt {
        Ok(response) => response.error_for_status(),
        Err(error) => Err(error),
    }
}

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template: &BosSessionTemplate,
) -> Result<Value, Box<dyn std::error::Error>> {
    log::debug!(
        "BOS sessiontemplate creation request payload:\n{}",
        serde_json::to_string_pretty(bos_template).unwrap()
    );

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

    let api_url = shasta_base_url.to_string() + "/bos/v1/sessiontemplate";

    let resp = client
        .post(api_url)
        .bearer_auth(shasta_token)
        .json(&bos_template)
        .send()
        .await?;

    if resp.status().is_success() {
        let response = resp.json().await?;

        log::debug!(
            "BOS sessiontemplate creation response payload:\n{:#?}",
            response
        );
        Ok(response)
    } else {
        let response: String = resp.text().await?;
        log::error!("FAIL response: {:#?}", response);
        Err(response.into())
    }
}

/// Delete BOS session templates.
pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let api_url = shasta_base_url.to_owned() + "/bos/v1/sessiontemplate/" + bos_template_id;

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
