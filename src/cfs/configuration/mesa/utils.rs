use comfy_table::Table;
use serde_json::Value;

use crate::{
    cfs::configuration::shasta::r#struct::cfs_configuration_response::CfsConfigurationResponse, hsm,
};

use super::r#struct::Configuration;

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
    cfs_configuration_name_opt: Option<&String>,
    hsm_group_name_vec: &Vec<String>,
    limit_number_opt: Option<&u8>,
) -> Vec<CfsConfigurationResponse> {
    if let Some(cfs_configuration_name) = cfs_configuration_name_opt {
        cfs_configuration_vec
            .retain(|cfs_configuration| cfs_configuration.name.eq(cfs_configuration_name));
    } else {
        let cfs_components: Vec<Value> = if !hsm_group_name_vec.is_empty() {
            let hsm_group_members = hsm::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
            )
            .await;

            // Note: nodes can be configured calling the component APi directly (bypassing BOS
            // session API)
            crate::cfs::component::mesa::http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &hsm_group_members,
            )
            .await
            .unwrap()
        } else {
            Vec::new()
        };

        let desired_config: Vec<&str> = cfs_components
            .iter()
            .map(|cfs_component| cfs_component["desiredConfig"].as_str().unwrap())
            .collect();

        // We need BOS session templates to find an image created by SAT
        let bos_sessiontemplate_value_vec = crate::bos::template::shasta::http_client::get(
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

        /* println!(
            "DEBUG - BOS sessiontemplate:\n{:#?}",
            bos_sessiontemplates_value_vec
        ); */

        // We need CFS sessions to find images without a BOS session template
        let cfs_session_value_vec = crate::cfs::session::shasta::http_client::filter(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name_vec,
            None,
            None,
            Some(true),
        )
        .await
        .unwrap();

        let image_id_cfs_configuration_target_from_bos_sessiontemplate: Vec<(
            String,
            String,
            Vec<String>,
        )> = crate::bos::template::mesa::utils::get_image_id_cfs_configuration_target_tuple_vec(
            bos_sessiontemplate_value_vec,
        );

        let image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
            crate::cfs::session::shasta::utils::get_image_id_cfs_configuration_target_tuple_vec(
                cfs_session_value_vec,
            );

        let image_id_cfs_configuration_target: Vec<&str> = [
            image_id_cfs_configuration_target_from_bos_sessiontemplate
                .iter()
                .map(|(_, config, _)| config.as_str())
                .collect(),
            image_id_cfs_configuration_target_from_cfs_session
                .iter()
                .map(|(_, config, _)| config.as_str())
                .collect(),
            desired_config,
        ]
        .concat();

        cfs_configuration_vec.retain(|cfs_configuration| {
            hsm_group_name_vec
                .iter()
                .any(|hsm_group| cfs_configuration.name.contains(hsm_group))
                || image_id_cfs_configuration_target.contains(&cfs_configuration.name.as_str())
        });

        // println!("DEBUG - CFS session:\n{:#?}", cfs_session_vec);

        cfs_configuration_vec.sort_by(|cfs_configuration_1, cfs_configuration_2| {
            cfs_configuration_1
                .last_updated
                .cmp(&cfs_configuration_2.last_updated)
        });

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            *cfs_configuration_vec = cfs_configuration_vec[cfs_configuration_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
                .to_vec();
        }
    }

    cfs_configuration_vec.to_vec()
}

pub fn print_table(cfs_configuration: Configuration) {
    let mut table = Table::new();

    table.set_header(vec!["Name", "Last updated", "Layers"]);

    let mut layers: String = String::new();

    if !cfs_configuration.config_layers.is_empty() {
        layers = format!(
            "COMMIT ID: {} COMMIT DATE: {} NAME: {} AUTHOR: {}",
            cfs_configuration.config_layers[0].commit_id,
            cfs_configuration.config_layers[0].commit_date,
            cfs_configuration.config_layers[0].name,
            cfs_configuration.config_layers[0].author
        );

        for i in 1..cfs_configuration.config_layers.len() {
            let layer = &cfs_configuration.config_layers[i];
            layers = format!(
                "{}\nCOMMIT ID: {} COMMIT DATE: {} NAME: {} AUTHOR: {}",
                layers, layer.commit_id, layer.commit_date, layer.name, layer.author
            );
        }
    }

    table.add_row(vec![
        cfs_configuration.name,
        cfs_configuration.last_updated,
        layers,
    ]);

    println!("{table}");
}
