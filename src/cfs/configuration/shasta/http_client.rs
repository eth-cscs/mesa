use std::error::Error;

use crate::hsm;

use serde_json::Value;

use super::r#struct::cfs_configuration_request::CfsConfigurationRequest;

pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations";

    let network_response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match network_response_rslt {
        Ok(http_response) => http_response.error_for_status(),
        Err(network_error) => Err(network_error),
    }
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations";

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    let json_response: Value = if resp.status().is_success() {
        serde_json::from_str(&resp.text().await?)?
    } else {
        return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
    };

    let configuration_value_vec = json_response.as_array().unwrap().clone();

    log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

    Ok(configuration_value_vec)
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec_opt: Option<&Vec<String>>,
    configuration_name_opt: Option<&String>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let configuration_value_vec: Vec<Value> =
        get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap();

    log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

    filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_value_vec.clone(),
        hsm_group_name_vec_opt,
        configuration_name_opt,
        None,
        limit_number_opt,
    )
    .await
    .unwrap();

    log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

    Ok(configuration_value_vec)
}

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    mut configuration_value_vec: Vec<Value>,
    hsm_group_name_vec_opt: Option<&Vec<String>>,
    configuration_name_opt: Option<&String>,
    most_recent_opt: Option<bool>,
    limit_number_opt: Option<&u8>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    // FILTER BY HSM GROUP NAMES
    if !hsm_group_name_vec_opt.unwrap().is_empty() {
        if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
            let hsm_group_member_vec = hsm::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            let cfs_session_vec = crate::cfs::session::mesa::http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
                None,
                None,
                None,
            )
            .await
            .unwrap();

            /* println!("DEBUG - CFS SESSION");
            for cfs_session in &cfs_session_vec {
                println!(
                    "DEBUG - hsm_group {:?} cfs_configuration {:?}",
                    cfs_session.target.clone().unwrap().groups.unwrap(),
                    cfs_session.configuration
                );
            } */

            let cfs_configuration_name_vec_from_cfs_session = cfs_session_vec
                .iter()
                .map(|cfs_session| cfs_session.configuration.clone().unwrap().name.unwrap())
                .collect::<Vec<_>>();

            let bos_sessiontemplate_vec = crate::bos::template::mesa::http_client::get_all(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
            )
            .await
            .unwrap()
            .into_iter()
            .filter(|bos_sessiontemplate| {
                let boot_set_vec = bos_sessiontemplate
                    .clone()
                    .boot_sets
                    .clone()
                    .unwrap_or_default();

                let mut boot_set_node_groups_vec = boot_set_vec
                    .iter()
                    .flat_map(|boot_set| boot_set.clone().node_groups.clone().unwrap_or_default());

                let mut boot_set_node_list_vec = boot_set_vec
                    .iter()
                    .flat_map(|boot_set| boot_set.clone().node_list.clone().unwrap_or_default());

                boot_set_node_groups_vec.clone().count() > 0
                    && boot_set_node_groups_vec
                        .all(|node_group| hsm_group_name_vec.contains(&node_group))
                    || boot_set_node_list_vec.clone().count() > 0
                        && boot_set_node_list_vec.all(|xname| hsm_group_member_vec.contains(&xname))
            })
            .collect::<Vec<_>>();

            /* println!("DEBUG - BOS SESSIONTEMPLATE");
            for bos_sessiontemplate in &bos_sessiontemplate_vec {
                println!(
                    "DEBUG - hsm_group {:?} cfs_configuration {:?}",
                    bos_sessiontemplate
                        .clone()
                        .boot_sets
                        .unwrap()
                        .iter()
                        .flat_map(|boot_set| boot_set.node_groups.clone().unwrap_or_default())
                        .collect::<Vec<_>>(),
                    bos_sessiontemplate.cfs.clone().unwrap().configuration
                );
            } */

            let cfs_configuration_name_from_bos_sessiontemplate = bos_sessiontemplate_vec
                .iter()
                .map(|bos_sessiontemplate| {
                    bos_sessiontemplate
                        .cfs
                        .clone()
                        .unwrap()
                        .configuration
                        .clone()
                        .unwrap()
                })
                .collect::<Vec<_>>();

            let cfs_configuration_name_from_cfs_session_and_bos_settiontemplate = [
                cfs_configuration_name_vec_from_cfs_session,
                cfs_configuration_name_from_bos_sessiontemplate,
            ]
            .concat();

            /* println!(
                "DEBUG - cfs configuration names:\n{:#?}",
                cfs_configuration_name_from_cfs_session_and_bos_settiontemplate
            ); */

            configuration_value_vec.retain(|cfs_configuration| {
                cfs_configuration_name_from_cfs_session_and_bos_settiontemplate
                    .contains(&cfs_configuration["name"].as_str().unwrap().to_string())
            });

            /* println!(
                "DEBUG - cfs confguration:\n{:#?}",
                cfs_configuration_value_vec
            ); */
        }
    }
    // END FILTER BY HSM GROUP NAME
    if let Some(configuration_name) = configuration_name_opt {
        configuration_value_vec.retain(|cfs_configuration| {
            cfs_configuration["name"]
                .as_str()
                .unwrap()
                .eq(configuration_name)
        });
    }

    configuration_value_vec.sort_by(|a, b| {
        a["lastUpdated"]
            .as_str()
            .unwrap()
            .cmp(b["lastUpdated"].as_str().unwrap())
    });

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client

        configuration_value_vec = configuration_value_vec[configuration_value_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    }

    // println!("DEBUG - cfs configuration:\n{:#?}", configuration_value_vec.iter().map(|conf| conf["name"].clone()).collect::<Vec<_>>());

    if most_recent_opt.is_some() && most_recent_opt.unwrap() {
        configuration_value_vec = [configuration_value_vec.first().unwrap().clone()].to_vec();
    }

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

    let network_response_rslt = client
        .put(api_url)
        .json(&serde_json::json!({"layers": configuration.layers})) // Encapsulating configuration.layers
        .bearer_auth(shasta_token)
        .send()
        .await;

    match network_response_rslt {
        Ok(http_response) => http_response.error_for_status(),
        Err(network_error) => Err(network_error),
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
