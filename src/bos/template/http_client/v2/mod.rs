pub mod r#struct;

use serde_json::Value;

use crate::{bos::template::http_client::v2::r#struct::BosSessionTemplate, error::Error};

/// Get BOS session templates. Ref --> https://apidocs.svc.cscs.ch/paas/bos/operation/get_v1_sessiontemplates/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&str>,
) -> Result<Vec<BosSessionTemplate>, Error> {
    log::info!("Get BOS sessiontemplate {:?}", bos_session_template_id_opt);

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
        shasta_base_url.to_owned() + "/bos/v2/sessiontemplates/" + bos_session_template_id
    } else {
        shasta_base_url.to_owned() + "/bos/v2/sessiontemplates"
    };

    let response = client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        if bos_session_template_id_opt.is_none() {
            response
                .json()
                .await
                .map_err(|error| Error::NetError(error))
        } else {
            response
                .json::<BosSessionTemplate>()
                .await
                .map(|cfs_configuration| vec![cfs_configuration])
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

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<BosSessionTemplate>, Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}

pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template: &BosSessionTemplate,
    bos_template_name: &str,
) -> Result<BosSessionTemplate, Error> {
    log::info!("Create BOS sessiontemplte '{}'", bos_template_name);
    log::debug!(
        "Create BOS sessiontemplate request payload:\n{}",
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

    let api_url = format!(
        "{}/bos/v2/sessiontemplates/{}",
        shasta_base_url, bos_template_name
    );

    let response = client
        .put(api_url)
        .json(&bos_template)
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
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}

/// Delete BOS session templates.
pub async fn delete(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template_id: &str,
) -> Result<(), reqwest::Error> {
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

    let api_url = shasta_base_url.to_owned() + "/bos/v2/sessiontemplates/" + bos_template_id;

    let _ = client
        .delete(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await?
        .error_for_status();

    Ok(())
}
