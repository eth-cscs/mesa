use serde_json::Value;

use crate::cfs::configuration::mesa::r#struct::{
    cfs_configuration_request::CfsConfigurationRequest,
    cfs_configuration_response::CfsConfigurationResponse,
};

use super::r#struct::cfs_configuration_response::ApiError;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
) -> Result<Vec<CfsConfigurationResponse>, reqwest::Error> {
    let response_rslt = crate::cfs::configuration::shasta::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_name_opt,
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

    cfs_configuration_vec.sort_by(|a, b| a.last_updated.cmp(&b.last_updated));

    Ok(cfs_configuration_vec)
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get_and_filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name: Option<&str>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    let mut cfs_configuration_value_vec: Vec<CfsConfigurationResponse> =
        crate::cfs::configuration::mesa::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            configuration_name,
        )
        .await
        .unwrap_or_default();

    crate::cfs::configuration::mesa::utils::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_configuration_value_vec,
        hsm_group_name_vec,
        limit_number_opt,
    )
    .await
}

pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<CfsConfigurationResponse, ApiError> {
    // Check if CFS configuration already exists
    let cfs_configuration_rslt = get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        Some(configuration_name),
    )
    .await;

    if cfs_configuration_rslt.is_ok_and(|cfs_configuration_vec| !cfs_configuration_vec.is_empty()) {
        return Err(ApiError::MesaError(format!("CFS configuration '{}' already exists.", configuration_name)));
        // return Err(serde_json::json!(format!(
        //     "ERROR: CFS configuration '{}' already exists",
        //     configuration_name
        // )));
    }

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
        let cfs_configuration: CfsConfigurationResponse =
            cfs_configuration_response.json().await.unwrap();
        Ok(cfs_configuration)
    } else {
        let error_detail = cfs_configuration_response.json::<serde_json::Value>().await.unwrap()["detail"]
            .as_str()
            .unwrap()
            .to_string();
        Err(ApiError::CsmError(error_detail))
    }
}
