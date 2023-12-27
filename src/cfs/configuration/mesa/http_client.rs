use serde_json::Value;

use crate::cfs::configuration::shasta::r#struct::{
    cfs_configuration_request::CfsConfigurationRequest,
    cfs_configuration_response::CfsConfigurationResponse,
};

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&String>,
    // limit_number_opt: Option<&u8>,
) -> Result<Vec<CfsConfigurationResponse>, reqwest::Error> {
    let response_rslt = crate::cfs::configuration::shasta::http_client::get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_name_opt.map(|config| config.as_str()),
    )
    .await;

    let mut cfs_configuration_vec: Vec<CfsConfigurationResponse> = match response_rslt {
        Ok(response) => {
            if configuration_name_opt.is_none() {
                response
                    .json::<Vec<CfsConfigurationResponse>>()
                    .await
                    .unwrap()
            } else {
                vec![response.json::<CfsConfigurationResponse>().await.unwrap()]
            }
        }
        Err(error) => return Err(error),
    };

    /* let mut cfs_configuration_vec: Vec<CfsConfigurationResponse>;

    if cfs_configuration_response.status().is_success() {
        cfs_configuration_vec = cfs_configuration_response.json().await.unwrap();
    } else {
        return Err(cfs_configuration_response.json().await.unwrap());
    } */

    cfs_configuration_vec.sort_by(|a, b| a.last_updated.cmp(&b.last_updated));

    /* if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        cfs_configuration_vec = cfs_configuration_vec[cfs_configuration_vec
            .len()
            .saturating_sub(*limit_number as usize)..]
            .to_vec();
    } */

    Ok(cfs_configuration_vec)
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get_and_filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name: Option<&String>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    /* let cfs_configuration_value_vec = shasta::cfs::configuration::http_client::get_all(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
    )
    .await
    .unwrap_or_default(); */

    let mut cfs_configuration_value_vec: Vec<CfsConfigurationResponse> =
        crate::cfs::configuration::mesa::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
        )
        .await
        .unwrap_or_default();

    crate::cfs::configuration::mesa::utils::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_configuration_value_vec,
        configuration_name,
        hsm_group_name_vec,
        limit_number_opt,
    )
    .await

    /* shasta::cfs::configuration::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        cfs_configuration_value_vec,
        Some(hsm_group_name_vec),
        configuration_name,
        most_recent_opt,
        limit_number_opt,
    )
    .await
    .unwrap() */
}

pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<CfsConfigurationResponse, Value> {
    let cfs_configuration_response = crate::cfs::configuration::shasta::http_client::put_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration,
        configuration_name,
    )
    .await;

    /* if cfs_configuration_response.is_err() {
        println!(
            "DEBUG - ERROR creating cfs configuration:\n{:#?}",
            cfs_configuration_response.as_ref().unwrap_err()
        )
    } */

    let cfs_configuration_response = cfs_configuration_response.unwrap();

    if cfs_configuration_response.status().is_success() {
        let cfs_configuration: CfsConfigurationResponse =
            cfs_configuration_response.json().await.unwrap();
        Ok(cfs_configuration)
    } else {
        Err(cfs_configuration_response.json().await.unwrap())
    }
}
