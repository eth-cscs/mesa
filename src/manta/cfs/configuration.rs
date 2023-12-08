use comfy_table::Table;
use std::fmt;

use crate::{shasta::cfs::configuration::get_put_payload::CfsConfigurationResponse, mesa};

pub struct Configuration {
    pub name: String,
    pub last_updated: String,
    pub config_layers: Vec<Layer>,
}

impl Configuration {
    pub fn new(name: &str, last_updated: &str, config_layers: Vec<Layer>) -> Self {
        Self {
            name: String::from(name),
            last_updated: String::from(last_updated),
            config_layers,
        }
    }
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nConfig Details:\n - name: {}\n - last updated: {}\nLayers:",
            self.name, self.last_updated
        )?;

        for (i, config_layer) in self.config_layers.iter().enumerate() {
            write!(f, "\n Layer {}:{}", i, config_layer)?;
        }

        Ok(())
    }
}

pub struct Layer {
    pub name: String,
    pub repo_name: String,
    pub commit_id: String,
    pub author: String,
    pub commit_date: String,
}

impl Layer {
    pub fn new(
        name: &str,
        repo_name: &str,
        commit_id: &str,
        author: &str,
        commit_date: &str,
    ) -> Self {
        Self {
            name: String::from(name),
            repo_name: String::from(repo_name),
            commit_id: String::from(commit_id),
            author: String::from(author),
            commit_date: String::from(commit_date),
        }
    }
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
pub async fn get_configuration(
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

    let mut cfs_configuration_value_vec = mesa::cfs::configuration::http_client::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
    )
    .await
    .unwrap_or_default();

    mesa::cfs::configuration::http_client::utils::filter(
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

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n - name: {}\n - repo name: {}\n - commit id: {}\n - commit date: {}\n - author: {}",
            self.name, self.repo_name, self.commit_id, self.commit_date, self.author
        )
    }
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
