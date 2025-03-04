use backend_dispatcher::types::cfs::cfs_configuration_request::{
    AdditionalInventory as FrontEndAdditionalInventory,
    CfsConfigurationRequest as FrontEndCfsConfigurationRequest, Layer as FrontEndLayer,
    SpecialParameter as FrontEndSpecialParameter,
};
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
    pub playbook: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_parameters: Option<Vec<SpecialParameter>>,
}

impl From<FrontEndLayer> for Layer {
    fn from(front_end_layer: FrontEndLayer) -> Self {
        Self {
            name: front_end_layer.name,
            clone_url: front_end_layer.clone_url,
            source: front_end_layer.source,
            playbook: front_end_layer.playbook,
            commit: front_end_layer.commit,
            branch: front_end_layer.branch,
            special_parameters: front_end_layer
                .special_parameters
                .map(|special_parameters| {
                    special_parameters
                        .into_iter()
                        .map(|special_parameter| special_parameter.into())
                        .collect()
                }),
        }
    }
}

impl Into<FrontEndLayer> for Layer {
    fn into(self) -> FrontEndLayer {
        FrontEndLayer {
            name: self.name,
            clone_url: self.clone_url,
            source: self.source,
            playbook: self.playbook,
            commit: self.commit,
            branch: self.branch,
            special_parameters: self.special_parameters.map(|special_parameters| {
                special_parameters
                    .into_iter()
                    .map(|special_parameter| special_parameter.into())
                    .collect()
            }),
        }
    }
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
    pub ims_required_dkms: Option<bool>,
}

impl From<FrontEndSpecialParameter> for SpecialParameter {
    fn from(front_end_special_parameter: FrontEndSpecialParameter) -> Self {
        Self {
            ims_required_dkms: front_end_special_parameter.ims_required_dkms,
        }
    }
}

impl Into<FrontEndSpecialParameter> for SpecialParameter {
    fn into(self) -> FrontEndSpecialParameter {
        FrontEndSpecialParameter {
            ims_required_dkms: self.ims_required_dkms,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdditionalInventory {
    pub name: Option<String>,
    pub clone_url: String,
    pub source: Option<String>,
    pub commit: Option<String>,
    pub branch: Option<String>,
}

impl From<FrontEndAdditionalInventory> for AdditionalInventory {
    fn from(front_end_additional_inventory: FrontEndAdditionalInventory) -> Self {
        Self {
            name: front_end_additional_inventory.name,
            clone_url: front_end_additional_inventory.clone_url,
            source: front_end_additional_inventory.source,
            commit: front_end_additional_inventory.commit,
            branch: front_end_additional_inventory.branch,
        }
    }
}

impl Into<FrontEndAdditionalInventory> for AdditionalInventory {
    fn into(self) -> FrontEndAdditionalInventory {
        FrontEndAdditionalInventory {
            name: self.name,
            clone_url: self.clone_url,
            source: self.source,
            commit: self.commit,
            branch: self.branch,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfsConfigurationRequest {
    pub description: Option<String>,
    pub layers: Option<Vec<Layer>>,
    pub additional_inventory: Option<AdditionalInventory>,
}

impl From<FrontEndCfsConfigurationRequest> for CfsConfigurationRequest {
    fn from(front_end_cfs_configuration_request: FrontEndCfsConfigurationRequest) -> Self {
        Self {
            description: front_end_cfs_configuration_request.description,
            layers: front_end_cfs_configuration_request
                .layers
                .map(|layer_vec| layer_vec.into_iter().map(Layer::from).collect()),
            additional_inventory: front_end_cfs_configuration_request
                .additional_inventory
                .map(|additional_inventory| additional_inventory.into()),
        }
    }
}

impl Into<FrontEndCfsConfigurationRequest> for CfsConfigurationRequest {
    fn into(self) -> FrontEndCfsConfigurationRequest {
        FrontEndCfsConfigurationRequest {
            description: self.description,
            layers: self
                .layers
                .map(|layer_vec| layer_vec.into_iter().map(Layer::into).collect()),
            additional_inventory: self
                .additional_inventory
                .map(|additional_inventory| additional_inventory.into()),
        }
    }
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
        site_name: &str,
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
                        site_name,
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

    pub async fn create_from_repos(
        gitea_token: &str,
        gitea_base_url: &str,
        shasta_root_cert: &[u8],
        // repos: Vec<PathBuf>,
        repo_name_vec: Vec<String>,
        local_git_commit_vec: Vec<String>,
        playbook_file_name_opt: Option<&String>,
    ) -> Result<CfsConfigurationRequest, Error> {
        // Create CFS configuration
        let mut cfs_configuration = CfsConfigurationRequest::new();
        // cfs_configuration.name = cfs_configuration_name.to_string();

        /* for repo_path in &repos {
            // Get repo from path
            let repo = match local_git_repo::get_repo(&repo_path.to_string_lossy()) {
                Ok(repo) => repo,
                Err(_) => {
                    eprintln!(
                        "Could not find a git repo in {}",
                        repo_path.to_string_lossy()
                    );
                    std::process::exit(1);
                }
            };

            // Get last (most recent) commit
            let local_last_commit = local_git_repo::get_last_commit(&repo).unwrap();

            // Get repo name
            let repo_ref_origin = repo.find_remote("origin").unwrap();

            log::info!("Repo ref origin URL: {}", repo_ref_origin.url().unwrap());

            let repo_ref_origin_url = repo_ref_origin.url().unwrap();

            let repo_name = repo_ref_origin_url
                .substring(
                    repo_ref_origin_url.rfind(|c| c == '/').unwrap() + 1, // repo name should not include URI '/' separator
                    repo_ref_origin_url.len(), // repo_ref_origin_url.rfind(|c| c == '.').unwrap(),
                )
                .trim_end_matches(".git");

            let repo_name = "cray/".to_owned() + repo_name;

            // Check if repo and local commit id exists in Shasta cvs
            let shasta_commitid_details_resp = gitea::http_client::get_commit_details(
                "https://api-gw-service-nmn.local/vcs/",
                &repo_name,
                &local_last_commit.id().to_string(),
                gitea_token,
                shasta_root_cert,
            )
            .await;

            // Check sync status between user face and shasta VCS
            let _ = match shasta_commitid_details_resp {
                Ok(_) => {
                    log::debug!(
                        "Local latest commit id {} for repo {} exists in shasta",
                        local_last_commit.id(),
                        repo_name
                    );
                    shasta_commitid_details_resp.unwrap()
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            let clone_url = gitea_base_url.to_owned() + &repo_name;

            log::debug!("clone url: {}", clone_url);

            // Create CFS layer
            let cfs_layer = Layer::new(
                Some(format!(
                    "{}-{}",
                    repo_name,
                    chrono::offset::Local::now().timestamp()
                )),
                // Some(shasta_commitid_details["sha"].as_str().unwrap().to_string()),
                Some(clone_url),
                None,
                playbook_file_name_opt
                    .unwrap_or(&"site.yml".to_string())
                    .to_string(),
                Some(local_last_commit.id().to_string()),
                None,
                None,
            );

            CfsConfigurationRequest::add_layer(&mut cfs_configuration, cfs_layer);
        } */
        for (repo_name, local_last_commit) in repo_name_vec.iter().zip(local_git_commit_vec.iter())
        {
            // Check if repo and local commit id exists in Shasta cvs
            let shasta_commitid_details_resp = gitea::http_client::get_commit_details(
                "https://api-gw-service-nmn.local/vcs/",
                &repo_name,
                &local_last_commit,
                gitea_token,
                shasta_root_cert,
            )
            .await;

            // Check sync status between user face and shasta VCS
            let _ = match shasta_commitid_details_resp {
                Ok(_) => {
                    log::debug!(
                        "Local latest commit id {} for repo {} exists in shasta",
                        local_last_commit,
                        repo_name
                    );
                    shasta_commitid_details_resp.unwrap()
                }
                Err(e) => {
                    return Err(Error::Message(e.to_string()));
                    /* eprintln!("{}", e);
                    std::process::exit(1); */
                }
            };

            let clone_url = gitea_base_url.to_owned() + &repo_name;

            log::debug!("clone url: {}", clone_url);

            // Create CFS layer
            let cfs_layer = Layer::new(
                Some(format!(
                    "{}-{}",
                    repo_name,
                    chrono::offset::Local::now().timestamp()
                )),
                // Some(shasta_commitid_details["sha"].as_str().unwrap().to_string()),
                Some(clone_url),
                None,
                playbook_file_name_opt
                    .unwrap_or(&"site.yml".to_string())
                    .to_string(),
                Some(local_last_commit.to_string()),
                None,
                None,
            );

            CfsConfigurationRequest::add_layer(&mut cfs_configuration, cfs_layer);
        }

        log::debug!("CFS configuration:\n{:#?}", cfs_configuration);

        Ok(cfs_configuration)
    }
}
