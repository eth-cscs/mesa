use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{common::gitea, error::Error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layer {
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub source: Option<String>,
    playbook: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_parameters: Option<Vec<SpecialParameter>>,
}

impl Layer {
    pub fn new(
        name: Option<String>,
        clone_url: Option<String>,
        source: Option<String>,
        playbook: String,
        commit: Option<String>,
        branch: Option<String>,
        special_parameters: Option<Vec<SpecialParameter>>,
    ) -> Self {
        Self {
            clone_url,
            commit,
            name,
            playbook,
            branch,
            special_parameters,
            source,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecialParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    ims_required_dkms: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdditionalInventory {
    name: Option<String>,
    clone_url: String,
    source: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfsConfigurationRequest {
    pub description: Option<String>,
    pub layers: Option<Vec<Layer>>,
    pub additional_inventory: Option<AdditionalInventory>,
}

impl Default for CfsConfigurationRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl CfsConfigurationRequest {
    pub fn new() -> Self {
        Self {
            description: None,
            layers: Some(Vec::default()),
            additional_inventory: None,
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        if let Some(ref mut layers) = self.layers.as_mut() {
            layers.push(layer);
        }
    }

    pub async fn from_sat_file_serde_yaml(
        shasta_root_cert: &[u8],
        gitea_base_url: &str,
        gitea_token: &str,
        configuration_yaml: &serde_yaml::Value,
        cray_product_catalog: &BTreeMap<String, String>,
    ) -> Result<(String, Self), Error> {
        let cfs_configuration_name;
        let mut cfs_configuration = Self::new();

        cfs_configuration_name = configuration_yaml["name"].as_str().unwrap().to_string();

        for layer_yaml in configuration_yaml["layers"].as_sequence().unwrap() {
            // println!("DEBUG - ### Layer:\n{:#?}\n", layer_yaml);

            if layer_yaml.get("git").is_some() {
                // Git layer

                let layer_name = layer_yaml["name"].as_str().unwrap().to_string();

                let repo_url = layer_yaml["git"]["url"].as_str().unwrap().to_string();

                let commit_id_value_opt = layer_yaml["git"].get("commit");
                let tag_value_opt = layer_yaml["git"].get("tag");
                let branch_value_opt = layer_yaml["git"].get("branch");

                let commit_id_opt: Option<String> = if commit_id_value_opt.is_some() {
                    // Git commit id
                    layer_yaml["git"]
                        .get("commit")
                        .map(|commit_id| commit_id.as_str().unwrap().to_string())
                } else if let Some(git_tag_value) = tag_value_opt {
                    // Git tag
                    let git_tag = git_tag_value.as_str().unwrap();

                    log::info!("git tag: {}", git_tag_value.as_str().unwrap());

                    let tag_details_rslt = gitea::http_client::get_tag_details(
                        &repo_url,
                        git_tag,
                        gitea_token,
                        shasta_root_cert,
                    )
                    .await;

                    let tag_details = if let Ok(tag_details) = tag_details_rslt {
                        log::debug!("tag details:\n{:#?}", tag_details);
                        tag_details
                    } else {
                        return Err(Error::Message(
                            format!("ERROR - Could not get details for git tag '{}' in CFS configuration '{}'. Reason:\n{:#?}", git_tag, cfs_configuration_name, tag_details_rslt)
                        ));
                    };

                    // Assumming user sets an existing tag name. It could be an annotated tag
                    // (different object than the commit id with its own sha value) or a
                    // lightweight tag (pointer to commit id, therefore the tag will have the
                    // same sha as the commit id it points to), either way CFS session will
                    // do a `git checkout` to the sha we found here, if an annotated tag, then,
                    // git is clever enough to take us to the final commit id, if it is a
                    // lighweight tag, then there is no problem because the sha is the same
                    // as the commit id
                    // NOTE: the `id` field is the tag's sha, note we are not taking the commit id
                    // the tag points to and we should not use sha because otherwise we won't be
                    // able to fetch the annotated tag using a commit sha through the Gitea APIs
                    tag_details["id"].as_str().map(|commit| commit.to_string())
                } else if branch_value_opt.is_some() {
                    // Branch name
                    Some(
                        gitea::http_client::get_commit_pointed_by_branch(
                            gitea_base_url,
                            gitea_token,
                            shasta_root_cert,
                            &repo_url,
                            branch_value_opt.unwrap().as_str().unwrap(),
                        )
                        .await
                        .unwrap(),
                    )
                } else {
                    // This should be an error but we will let CSM to handle this
                    None
                };

                // IMPORTANT: CSM won't allow CFS configuration layers with both commit id and
                // branch name, therefore, we will set branch name to None if we already have a
                // commit id
                let branch_name = if commit_id_opt.is_some() {
                    None
                } else {
                    branch_value_opt.map(|branch_value| branch_value.as_str().unwrap().to_string())
                };

                let layer = Layer::new(
                    Some(layer_name),
                    Some(repo_url),
                    layer_yaml["source"]
                        .as_str()
                        .and_then(|source_value| Some(source_value.to_string())),
                    layer_yaml["playbook"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    commit_id_opt,
                    branch_name,
                    None,
                );
                cfs_configuration.add_layer(layer);
            } else if layer_yaml.get("product").is_some() {
                // Product layer

                let product_name = layer_yaml["product"]["name"].as_str().unwrap();
                let product_version = layer_yaml["product"]["version"].as_str().unwrap();
                let product_branch_value_opt = layer_yaml["product"].get("branch");
                let product_commit_value_opt = layer_yaml["product"].get("commit");

                let product = cray_product_catalog.get(product_name);

                if product.is_none() {
                    return Err(Error::Message(format!(
                        "Product {} not found in cray product catalog",
                        product_name
                    )));
                }

                let cos_cray_product_catalog =
                    serde_yaml::from_str::<Value>(product.unwrap()).unwrap();

                let product_details_opt = cos_cray_product_catalog
                    .get(product_version)
                    .and_then(|product| product.get("configuration"));

                if product_details_opt.is_none() {
                    return Err(Error::Message(
                        format!("Product details for product name '{}', product_version '{}' and 'configuration' not found in cray product catalog", product_name, product_version)
                    ));
                }

                let product_details = product_details_opt.unwrap().clone();

                log::debug!(
                    "CRAY product catalog details for product: {}, version: {}:\n{:#?}",
                    product_name,
                    product_version,
                    product_details
                );

                // Manta may run outside the CSM local network therefore we have to change the
                // internal URLs for the external one
                let repo_url = product_details["clone_url"]
                    .as_str()
                    .unwrap()
                    .to_string()
                    .replace("vcs.cmn.alps.cscs.ch", "api-gw-service-nmn.local");

                let commit_id_opt = if let Some(commit_value) = product_commit_value_opt {
                    commit_value
                        .clone()
                        .as_str()
                        .map(|commit_str| commit_str.to_string())
                } else {
                    if product_branch_value_opt.is_some() {
                        // If branch is provided, then ignore the commit id in the CRAY products table
                        Some(
                            gitea::http_client::get_commit_pointed_by_branch(
                                gitea_base_url,
                                gitea_token,
                                shasta_root_cert,
                                &repo_url,
                                product_branch_value_opt.unwrap().as_str().unwrap(),
                            )
                            .await
                            .unwrap(),
                        )
                    } else {
                        Some(product_details["commit"].as_str().unwrap().to_string())
                    }
                };

                // IMPORTANT: CSM won't allow CFS configuration layers with both commit id and
                // branch name, therefore, we will set branch name to None if we already have a
                // commit id
                let branch_name = if commit_id_opt.is_some() {
                    None
                } else {
                    product_branch_value_opt
                        .map(|branch_value| branch_value.as_str().unwrap().to_string())
                };

                // Create CFS configuration layer struct
                let layer = Layer::new(
                    Some(product_name.to_string()),
                    Some(repo_url),
                    layer_yaml["source"]
                        .as_str()
                        .map(|source_value| source_value.to_string()),
                    layer_yaml["playbook"].as_str().unwrap().to_string(),
                    commit_id_opt,
                    branch_name,
                    None,
                );
                cfs_configuration.add_layer(layer);
            } else {
                return Err(Error::Message(
                    format!("ERROR - configurations section in SAT file error - CFS configuration layer error")
                ));
            }
        }

        Ok((cfs_configuration_name, cfs_configuration))
    }
}
