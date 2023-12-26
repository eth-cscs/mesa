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

    let network_response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match network_response_rslt {
        Ok(http_response) => http_response.error_for_status(),
        Err(network_err) => Err(network_err),
    }
}

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&String>,
) -> Result<Vec<Value>, reqwest::Error> {
    let response = get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        bos_session_template_id_opt,
    )
    .await;

    let bos_session_template_response_value: Value = match response {
        Ok(cfs_session_template_value) => cfs_session_template_value.json().await.unwrap(),
        Err(error) => return Err(error),
    };

    let mut bos_session_template_vec = Vec::new();

    if bos_session_template_response_value.is_array() {
        for bos_session_template_value in bos_session_template_response_value.as_array().unwrap() {
            bos_session_template_vec.push(bos_session_template_value.clone());
        }
    } else {
        bos_session_template_vec.push(bos_session_template_response_value);
    }

    Ok(bos_session_template_vec)

    /* let json_response: Value = if resp.status().is_success() {
        serde_json::from_str(&resp.text().await?)?
    } else {
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    Ok(json_response.as_array().unwrap_or(&Vec::new()).to_vec()) */
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
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

    let api_url = shasta_base_url.to_owned() + "/bos/v1/sessiontemplate";

    let resp = client
        .get(api_url)
        // .get(format!("{}{}", shasta_base_url, "/bos/v1/sessiontemplate"))
        .bearer_auth(shasta_token)
        .send()
        .await?;

    let json_response: Value = if resp.status().is_success() {
        serde_json::from_str(&resp.text().await?)?
    } else {
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    Ok(json_response.as_array().unwrap_or(&Vec::new()).to_vec())
}

pub async fn get_and_filter(
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

    filter(
        &mut bos_sessiontemplate_value_vec,
        hsm_group_name_vec,
        bos_sessiontemplate_name_opt,
        cfs_configuration_name_vec_opt,
        limit_number_opt,
    )
    .await;

    Ok(bos_sessiontemplate_value_vec)
}

/// Get BOS session templates. Ref --> https://apidocs.svc.cscs.ch/paas/bos/operation/get_v1_sessiontemplates/
pub async fn filter(
    bos_sessiontemplate_value_vec: &mut Vec<Value>,
    hsm_group_name_vec: &Vec<String>,
    bos_sessiontemplate_name_opt: Option<&String>,
    cfs_configuration_name_vec_opt: Option<Vec<&str>>,
    limit_number_opt: Option<&u8>,
) {
    if !hsm_group_name_vec.is_empty() {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate_value| {
            bos_sessiontemplate_value["boot_sets"]
                .as_object()
                .is_some_and(|boot_set_obj| {
                    boot_set_obj.iter().any(|(_property, boot_set_param)| {
                        boot_set_param["node_groups"]
                            .as_array()
                            .is_some_and(|node_group_vec| {
                                node_group_vec.iter().any(|node_group| {
                                    hsm_group_name_vec
                                        .contains(&node_group.as_str().unwrap().to_string())
                                })
                            })
                    })
                })
        });
    }

    if let Some(cfs_configuration_name_vec) = cfs_configuration_name_vec_opt {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate_value| {
            cfs_configuration_name_vec.contains(
                &bos_sessiontemplate_value
                    .pointer("/cfs/configuration")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
        });
    }

    if let Some(bos_sessiontemplate_name) = bos_sessiontemplate_name_opt {
        bos_sessiontemplate_value_vec.retain(|bos_sessiontemplate| {
            bos_sessiontemplate["name"]
                .as_str()
                .unwrap()
                .eq(bos_sessiontemplate_name)
        });
    }

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client

        *bos_sessiontemplate_value_vec = bos_sessiontemplate_value_vec
            [bos_sessiontemplate_value_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }
}

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
