pub mod http_client {

    use serde_json::Value;

    use crate::{
        mesa::cfs::configuration::get_put_payload::{self, CfsConfigurationResponse},
        shasta::{self, cfs::configuration::CfsConfigurationRequest},
    };

    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        // hsm_group_name: Option<&String>,
        configuration_name: Option<&String>,
    ) -> Result<Vec<get_put_payload::CfsConfigurationResponse>, Value> {
        let cfs_configuration_response = shasta::cfs::configuration::http_client::get_raw(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
        )
        .await
        .unwrap();

        let mut cfs_configuration_vec: Vec<get_put_payload::CfsConfigurationResponse> = Vec::new();

        if cfs_configuration_response.status().is_success() {
            let cfs_configuration: get_put_payload::CfsConfigurationResponse =
                cfs_configuration_response.json().await.unwrap();
            cfs_configuration_vec.push(cfs_configuration);
            Ok(cfs_configuration_vec)
        } else {
            Err(cfs_configuration_response.json().await.unwrap())
        }
    }

    pub async fn put(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration: &CfsConfigurationRequest,
        configuration_name: &str,
    ) -> Result<CfsConfigurationResponse, Value> {
        let cfs_configuration_response = shasta::cfs::configuration::http_client::put_raw(
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
            Err(cfs_configuration_response.json().await.unwrap())
        }
    }
}

pub mod utils {
    use crate::mesa::cfs::configuration::get_put_payload::CfsConfigurationResponse;

    pub fn filter(
        cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
        cfs_configuration_name_opt: Option<&String>,
        limit_number: Option<&u8>,
    ) -> Vec<CfsConfigurationResponse> {
        if let Some(cfs_configuration_name) = cfs_configuration_name_opt {
            cfs_configuration_vec
                .retain(|cfs_configuration| cfs_configuration.name.eq(cfs_configuration_name));
        }

        cfs_configuration_vec.sort_by(|cfs_session_1, cfs_session_2| {
            cfs_session_1.last_updated.cmp(&cfs_session_2.last_updated)
        });

        if limit_number.is_some() {
            // Limiting the number of results to return to client

            *cfs_configuration_vec = cfs_configuration_vec[cfs_configuration_vec
                .len()
                .saturating_sub(*limit_number.unwrap() as usize)..]
                .to_vec();
        }

        cfs_configuration_vec.to_vec()
    }
}
