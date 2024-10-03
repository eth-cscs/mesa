/// Structs related to CFS confguration with data related to most recent commit id like, author
/// name, commit date, etc

pub mod v2 {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use serde_yaml::Value;

    use crate::common::gitea;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Layer {
        pub name: String,
        #[serde(rename = "cloneUrl")]
        pub clone_url: String,
        #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
        pub commit: Option<String>,
        playbook: String,
        #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
        pub branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(rename = "specialParameters")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub special_parameters: Option<Vec<SpecialParameter>>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SpecialParameter {
        #[serde(rename = "imsRequiredDkms")]
        #[serde(skip_serializing_if = "Option::is_none")]
        ims_required_dkms: Option<bool>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct CfsConfigurationRequest {
        pub name: String,
        pub layers: Vec<Layer>,
    }

    impl Layer {
        pub fn new(
            clone_url: String,
            commit: Option<String>,
            name: String,
            playbook: String,
            branch: Option<String>,
            tag: Option<String>,
            special_parameters: Option<Vec<SpecialParameter>>,
        ) -> Self {
            Self {
                clone_url,
                commit,
                name,
                playbook,
                branch,
                tag,
                special_parameters,
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
                name: String::default(),
                layers: Vec::default(),
            }
        }

        pub fn add_layer(&mut self, layer: Layer) {
            self.layers.push(layer);
        }

        pub async fn from_sat_file_serde_yaml(
            shasta_root_cert: &[u8],
            gitea_base_url: &str,
            gitea_token: &str,
            configuration_yaml: &serde_yaml::Value,
            cray_product_catalog: &BTreeMap<String, String>,
        ) -> Self {
            let mut cfs_configuration = Self::new();

            cfs_configuration.name = configuration_yaml["name"].as_str().unwrap().to_string();

            for layer_yaml in configuration_yaml["layers"].as_sequence().unwrap() {
                // println!("\n\n### Layer:\n{:#?}\n", layer_json);

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
                            eprintln!("ERROR - Could not get details for git tag '{}' in CFS configuration '{}'. Reason:\n{:#?}", git_tag, cfs_configuration.name, tag_details_rslt);
                            std::process::exit(1);
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
                        branch_value_opt
                            .map(|branch_value| branch_value.as_str().unwrap().to_string())
                    };

                    let layer = Layer::new(
                        repo_url,
                        commit_id_opt,
                        layer_name,
                        layer_yaml["playbook"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        branch_name,
                        None,
                        None,
                    );
                    cfs_configuration.add_layer(layer);
                } else if layer_yaml.get("product").is_some() {
                    // Product layer

                    let product_name = layer_yaml["product"]["name"].as_str().unwrap();
                    let product_version = layer_yaml["product"]["version"].as_str().unwrap();
                    let product_branch_value_opt = layer_yaml["product"].get("branch");

                    let product = cray_product_catalog.get(product_name);

                    if product.is_none() {
                        eprintln!("Product {} not found in cray product catalog", product_name);
                        std::process::exit(1);
                    }

                    let cos_cray_product_catalog =
                        serde_yaml::from_str::<Value>(product.unwrap()).unwrap();

                    let product_details_opt = cos_cray_product_catalog
                        .get(product_version)
                        .and_then(|product| product.get("configuration"));

                    if product_details_opt.is_none() {
                        eprintln!("Product details for product name '{}', product_version '{}' and 'configuration' not found in cray product catalog", product_name, product_version);
                        std::process::exit(1);
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

                    let commit_id_opt = if product_branch_value_opt.is_some() {
                        // If branch is provided, then ignore the commit id in the CRAY products table

                        let commit = Some(
                            gitea::http_client::get_commit_pointed_by_branch(
                                gitea_base_url,
                                gitea_token,
                                shasta_root_cert,
                                &repo_url,
                                product_branch_value_opt.unwrap().as_str().unwrap(),
                            )
                            .await
                            .unwrap(),
                        );

                        commit
                    } else {
                        Some(product_details["commit"].as_str().unwrap().to_string())
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
                        repo_url,
                        commit_id_opt,
                        product_name.to_string(),
                        layer_yaml["playbook"].as_str().unwrap().to_string(),
                        branch_name,
                        None,
                        None,
                    );
                    cfs_configuration.add_layer(layer);
                } else {
                    eprintln!("ERROR - configurations section in SAT file error - CFS configuration layer error");
                    std::process::exit(1);
                }
            }

            cfs_configuration
        }

        /* pub async fn create_from_repos(
            gitea_token: &str,
            gitea_base_url: &str,
            shasta_root_cert: &[u8],
            repos: Vec<PathBuf>,
            cfs_configuration_name: &String,
        ) -> Self {
            // Create CFS configuration
            let mut cfs_configuration = CfsConfigurationRequest::new();
            cfs_configuration.name = cfs_configuration_name.to_string();

            for repo_path in &repos {
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

                let repo_name = repo_ref_origin_url.substring(
                    repo_ref_origin_url.rfind(|c| c == '/').unwrap() + 1, // repo name should not include URI '/' separator
                    repo_ref_origin_url.len(), // repo_ref_origin_url.rfind(|c| c == '.').unwrap(),
                );

                let api_url = "cray/".to_owned() + repo_name;

                // Check if repo and local commit id exists in Shasta cvs
                let shasta_commitid_details_resp =
                    gitea::http_client::get_commit_details_from_internal_url(
                        &api_url,
                        // &format!("/cray/{}", repo_name),
                        &local_last_commit.id().to_string(),
                        gitea_token,
                        shasta_root_cert,
                    )
                    .await;

                // Check sync status between user face and shasta VCS
                let shasta_commitid_details: serde_json::Value = match shasta_commitid_details_resp
                {
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

                let clone_url = gitea_base_url.to_owned() + "/cray/" + repo_name;

                // Create CFS layer
                let cfs_layer = Layer::new(
                    clone_url,
                    Some(shasta_commitid_details["sha"].as_str().unwrap().to_string()),
                    format!(
                        "{}-{}",
                        repo_name.substring(0, repo_name.len()),
                        chrono::offset::Local::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    ),
                    String::from("site.yml"),
                    None,
                    None,
                    None,
                );

                CfsConfigurationRequest::add_layer(&mut cfs_configuration, cfs_layer);
            }

            cfs_configuration
        } */
    }
}

pub mod v3 {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use serde_yaml::Value;

    use crate::common::gitea;

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
        ) -> (String, Self) {
            let cfs_configuration_name;
            let mut cfs_configuration = Self::new();

            cfs_configuration_name = configuration_yaml["name"].as_str().unwrap().to_string();

            for layer_yaml in configuration_yaml["layers"].as_sequence().unwrap() {
                // println!("\n\n### Layer:\n{:#?}\n", layer_json);

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
                            eprintln!("ERROR - Could not get details for git tag '{}' in CFS configuration '{}'. Reason:\n{:#?}", git_tag, cfs_configuration_name, tag_details_rslt);
                            std::process::exit(1);
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
                        branch_value_opt
                            .map(|branch_value| branch_value.as_str().unwrap().to_string())
                    };

                    let layer = Layer::new(
                        Some(repo_url),
                        commit_id_opt,
                        Some(layer_name),
                        layer_yaml["playbook"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        branch_name,
                        None,
                        None,
                    );
                    cfs_configuration.add_layer(layer);
                } else if layer_yaml.get("product").is_some() {
                    // Product layer

                    let product_name = layer_yaml["product"]["name"].as_str().unwrap();
                    let product_version = layer_yaml["product"]["version"].as_str().unwrap();
                    let product_branch_value_opt = layer_yaml["product"].get("branch");

                    let product = cray_product_catalog.get(product_name);

                    if product.is_none() {
                        eprintln!("Product {} not found in cray product catalog", product_name);
                        std::process::exit(1);
                    }

                    let cos_cray_product_catalog =
                        serde_yaml::from_str::<Value>(product.unwrap()).unwrap();

                    let product_details_opt = cos_cray_product_catalog
                        .get(product_version)
                        .and_then(|product| product.get("configuration"));

                    if product_details_opt.is_none() {
                        eprintln!("Product details for product name '{}', product_version '{}' and 'configuration' not found in cray product catalog", product_name, product_version);
                        std::process::exit(1);
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

                    let commit_id_opt = if product_branch_value_opt.is_some() {
                        // If branch is provided, then ignore the commit id in the CRAY products table

                        let commit = Some(
                            gitea::http_client::get_commit_pointed_by_branch(
                                gitea_base_url,
                                gitea_token,
                                shasta_root_cert,
                                &repo_url,
                                product_branch_value_opt.unwrap().as_str().unwrap(),
                            )
                            .await
                            .unwrap(),
                        );

                        commit
                    } else {
                        Some(product_details["commit"].as_str().unwrap().to_string())
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
                        Some(repo_url),
                        commit_id_opt,
                        Some(product_name.to_string()),
                        layer_yaml["playbook"].as_str().unwrap().to_string(),
                        branch_name,
                        None,
                        None,
                    );
                    cfs_configuration.add_layer(layer);
                } else {
                    eprintln!("ERROR - configurations section in SAT file error - CFS configuration layer error");
                    std::process::exit(1);
                }
            }

            (cfs_configuration_name, cfs_configuration)
        }

        /* pub async fn create_from_repos(
            gitea_token: &str,
            gitea_base_url: &str,
            shasta_root_cert: &[u8],
            repos: Vec<PathBuf>,
            cfs_configuration_name: &String,
        ) -> Self {
            // Create CFS configuration
            let mut cfs_configuration = CfsConfigurationRequest::new();
            cfs_configuration.name = cfs_configuration_name.to_string();

            for repo_path in &repos {
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

                let repo_name = repo_ref_origin_url.substring(
                    repo_ref_origin_url.rfind(|c| c == '/').unwrap() + 1, // repo name should not include URI '/' separator
                    repo_ref_origin_url.len(), // repo_ref_origin_url.rfind(|c| c == '.').unwrap(),
                );

                let api_url = "cray/".to_owned() + repo_name;

                // Check if repo and local commit id exists in Shasta cvs
                let shasta_commitid_details_resp =
                    gitea::http_client::get_commit_details_from_internal_url(
                        &api_url,
                        // &format!("/cray/{}", repo_name),
                        &local_last_commit.id().to_string(),
                        gitea_token,
                        shasta_root_cert,
                    )
                    .await;

                // Check sync status between user face and shasta VCS
                let shasta_commitid_details: serde_json::Value = match shasta_commitid_details_resp
                {
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

                let clone_url = gitea_base_url.to_owned() + "/cray/" + repo_name;

                // Create CFS layer
                let cfs_layer = Layer::new(
                    clone_url,
                    Some(shasta_commitid_details["sha"].as_str().unwrap().to_string()),
                    format!(
                        "{}-{}",
                        repo_name.substring(0, repo_name.len()),
                        chrono::offset::Local::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    ),
                    String::from("site.yml"),
                    None,
                    None,
                    None,
                );

                CfsConfigurationRequest::add_layer(&mut cfs_configuration, cfs_layer);
            }

            cfs_configuration
        } */
    }
}
