use std::collections::HashMap;

use crate::hsm;

pub struct VCluster {
    pub name: String,
    pub description: String,
}

impl VCluster {
    pub async fn power_off(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name: &str,
        reason: Option<String>,
        force: bool,
    ) -> Result<(), reqwest::Error> {
        let hsm_group_node_list = hsm::group::shasta::utils::get_member_vec_from_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name,
        )
        .await;

        let _ = crate::capmc::http_client::node_power_off::post_sync(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_node_list,
            reason,
            force,
        )
        .await;

        Ok(())
    }

    pub async fn power_on(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name: &str,
        reason: Option<String>,
    ) -> Result<(), reqwest::Error> {
        let hsm_group_node_list = hsm::group::shasta::utils::get_member_vec_from_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name,
        )
        .await;

        let _ = crate::capmc::http_client::node_power_on::post_sync(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_node_list,
            reason,
        )
        .await;

        Ok(())
    }

    pub async fn power_reset(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name: &str,
        reason: Option<String>,
        force: bool,
    ) -> Result<(), reqwest::Error> {
        let hsm_group_node_list = hsm::group::shasta::utils::get_member_vec_from_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name,
        )
        .await;

        let _ = crate::capmc::http_client::node_power_reset::post_sync(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_node_list,
            reason,
            force,
        )
        .await;

        Ok(())
    }

    /// Returns a map with the xnames and the cfs configuration used to boot image
    pub async fn get_boot_configuration(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name: &str,
    ) -> Option<HashMap<String, String>> {
        let hsm_group_node_list = hsm::group::shasta::utils::get_member_vec_from_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name,
        )
        .await;

        let hsm_group_node_boot_param_vec =
            crate::bss::bootparameters::http_client::get_boot_params(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &hsm_group_node_list,
            )
            .await
            .unwrap();

        let mut node_image_map: HashMap<String, String> = HashMap::new();

        for boot_param in hsm_group_node_boot_param_vec {
            let image_id: String = boot_param.get_boot_image();

            for node in &boot_param.hosts {
                node_image_map
                    .entry(node.to_string())
                    .or_insert(image_id.clone());
            }
        }

        // Find CFS configuration for each image id. For this we need to fetch all CFS sessions and
        // all BOS sessiontemplates ...
        // TODO: create fn that receives a list of CFS sessions and a list of BOS sessiontemplates
        // and returns a list of tuples like (image id, cfs configuration name)

        Some(node_image_map)
    }

    pub async fn get_configuration() -> Option<String> {
        Some("".to_string())
    }
}
