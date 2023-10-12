pub mod http_client {

    use serde_json::Value;

    use crate::{mesa::cfs::configuration::get_put_payload::CfsConfiguration, shasta};

    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        // hsm_group_name: Option<&String>,
        configuration_name: Option<&String>,
        limit_number: Option<&u8>,
    ) -> Result<Vec<CfsConfiguration>, reqwest::Error> {
        let cfs_configuration_response = shasta::cfs::configuration::http_client::get_raw(
            shasta_token,
            shasta_base_url,
            configuration_name,
        )
        .await;

        let cfs_configuration_response_value: Value = match cfs_configuration_response {
            Ok(cfs_configuration_value) => cfs_configuration_value.json().await.unwrap(),
            Err(error) => return Err(error),
        };

        let cfs_configuration_vec: Vec<CfsConfiguration> =
            serde_json::from_value(cfs_configuration_response_value).unwrap();

        Ok(cfs_configuration_vec)
    }
}

pub mod utils {
    use crate::mesa::cfs::configuration::get_put_payload::CfsConfiguration;

    pub fn filter(
        shasta_token: &str,
        shasta_base_url: &str,
        cfs_configuration_vec: &mut Vec<CfsConfiguration>,
        cfs_configuration_name_opt: Option<&String>,
        limit_number: Option<&u8>,
    ) -> Vec<CfsConfiguration> {
        if let Some(cfs_configuration_name) = cfs_configuration_name_opt {
            cfs_configuration_vec
                .retain(|cfs_configuration| cfs_configuration.name.eq(cfs_configuration_name));
        }

        cfs_configuration_vec.sort_by(|cfs_session_1, cfs_session_2| {
            cfs_session_1.last_update.cmp(&cfs_session_2.last_update)
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
