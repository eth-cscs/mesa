pub mod csm;
pub mod utils;

use crate::{
    cfs::{
        self,
        configuration::csm::v3::r#struct::{
            cfs_configuration_request::CfsConfigurationRequest,
            cfs_configuration_response::CfsConfigurationResponse,
        },
    },
    error::Error,
};

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration_name_opt: Option<&str>,
) -> Result<Vec<CfsConfigurationResponse>, Error> {
    cfs::configuration::csm::v3::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration_name_opt,
    )
    .await
}

// This function enforces a new CFS configuration to be created. First, checks if CFS configuration
// with same name already exists in CSM, if that is the case, it will return an error, otherwise
// creates a new CFS configuration
pub async fn put(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    configuration: &CfsConfigurationRequest,
    configuration_name: &str,
) -> Result<CfsConfigurationResponse, Error> {
    // Check if CFS configuration already exists
    log::info!("Check CFS configuration '{}' exists", configuration_name);

    let cfs_configuration_rslt = get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        Some(configuration_name),
    )
    .await;

    // Check if CFS configuration already exists and throw an error is that is the case
    if cfs_configuration_rslt.is_ok_and(|cfs_configuration_vec| !cfs_configuration_vec.is_empty()) {
        return Err(Error::Message(format!(
            "CFS configuration '{}' already exists.",
            configuration_name
        )));
    }

    log::info!(
        "CFS configuration '{}' does not exists, creating new CFS configuration",
        configuration_name
    );

    cfs::configuration::csm::v3::http_client::put(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        configuration,
        configuration_name,
    )
    .await
}
