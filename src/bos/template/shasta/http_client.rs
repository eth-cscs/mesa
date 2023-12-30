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

/* pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&String>,
) -> Result<Vec<Value>, reqwest::Error> {
    let response_rslt = get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        bos_session_template_id_opt,
    )
    .await;

    let bos_session_template_vec: Vec<Value> = match response_rslt {
        Ok(response) => {
            if bos_session_template_id_opt.is_none() {
                response.json::<Vec<Value>>().await.unwrap()
            } else {
                vec![response.json::<Value>().await.unwrap()]
            }
        }
        Err(error) => return Err(error),
    };

    Ok(bos_session_template_vec)
} */

/* pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Value>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
} */

/* pub async fn get_and_filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &Vec<String>,
    bos_sessiontemplate_name_opt: Option<&String>,
    cfs_configuration_name_vec_opt: Option<Vec<&str>>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut bos_sessiontemplate_value_vec: Vec<Value> =
        get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap();

    crate::bos::template::shasta::utils::filter(
        &mut bos_sessiontemplate_value_vec,
        hsm_group_name_vec,
        bos_sessiontemplate_name_opt,
        cfs_configuration_name_vec_opt,
        limit_number_opt,
    )
    .await;

    Ok(bos_sessiontemplate_value_vec)
} */

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_template: &BosSessionTemplate,
) -> Result<Value, Box<dyn std::error::Error>> {
    log::debug!("Bos template:\n{:#?}", bos_template);

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
        log::debug!("Response:\n{:#?}", response);
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
