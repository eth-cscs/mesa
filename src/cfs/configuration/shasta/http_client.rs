use std::error::Error;

use serde_json::Value;

use super::r#struct::cfs_configuration_request::CfsConfigurationRequest;

pub async fn get_raw(
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

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
) -> Result<Vec<Value>, reqwest::Error> {
    let response_rslt = get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_name_opt,
    )
    .await;

    let mut cfs_configuration_value_vec: Vec<Value> = match response_rslt {
        Ok(response) => {
            if configuration_name_opt.is_none() {
                response.json::<Vec<Value>>().await.unwrap()
            } else {
                vec![response.json::<Value>().await.unwrap()]
            }
        }
        Err(error) => return Err(error),
    };

    cfs_configuration_value_vec.sort_by(|a, b| {
        a["lastUpdated"]
            .as_str()
            .unwrap()
            .cmp(b["lastUpdated"].as_str().unwrap())
    });

    Ok(cfs_configuration_value_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Value>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}

pub async fn get_and_filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
    hsm_group_name_vec_opt: Option<&Vec<String>>,
    most_recent_opt: Option<bool>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut configuration_value_vec = get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_name_opt,
    )
    .await
    .unwrap();

    crate::cfs::configuration::shasta::utils::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut configuration_value_vec,
        hsm_group_name_vec_opt,
        most_recent_opt,
        limit_number_opt,
    )
    .await;

    Ok(configuration_value_vec)
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

    let response_rslt = client
        .put(api_url)
        .json(&serde_json::json!({"layers": configuration.layers})) // Encapsulating configuration.layers
        .bearer_auth(shasta_token)
        .send()
        .await;

    match response_rslt {
        Ok(response) => response.error_for_status(),
        Err(error) => Err(error),
    }
}

pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<Value, Box<dyn Error>> {
    let cfs_configuration_response = crate::cfs::configuration::shasta::http_client::put_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration,
        configuration_name,
    )
    .await
    .unwrap();

    if cfs_configuration_response.status().is_success() {
        let response = &cfs_configuration_response.text().await?;
        log::debug!("CFS configuration creation response:\n{:#?}", response);
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", cfs_configuration_response);
        let response: String = cfs_configuration_response.text().await?;
        log::error!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
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
