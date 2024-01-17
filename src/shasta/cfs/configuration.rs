use std::path::PathBuf;

use k8s_openapi::chrono;
use serde::{Deserialize, Serialize};
use substring::Substring;

use crate::{
    common::{gitea, local_git_repo},
    shasta::cfs::configuration,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layer {
    #[serde(rename = "cloneUrl")]
    clone_url: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    commit: Option<String>,
    name: String,
    playbook: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)] // TODO: investigate why serde can Deserialize dynamically syzed structs `Vec<Layer>`
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
    ) -> Self {
        Self {
            clone_url,
            commit,
            name,
            playbook,
            branch,
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

    pub fn from_sat_file_serde_yaml(configuration_yaml: &serde_yaml::Value) -> Self {
        let mut cfs_configuration = Self::new();

        cfs_configuration.name = configuration_yaml["name"].as_str().unwrap().to_string();

        for layer_yaml in configuration_yaml["layers"].as_sequence().unwrap() {
            // println!("\n\n### Layer:\n{:#?}\n", layer_json);

            if layer_yaml.get("git").is_some() {
                // Git layer
                let repo_name = layer_yaml["name"].as_str().unwrap().to_string();
                let repo_url = layer_yaml["git"]["url"].as_str().unwrap().to_string();
                let layer = Layer::new(
                    repo_url,
                    layer_yaml["git"]["commit"]
                        .as_str()
                        .map(|commit| commit.to_string()),
                    repo_name,
                    layer_yaml["playbook"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    layer_yaml["git"]["branch"]
                        .as_str()
                        .map(|branch| branch.to_string()),
                );
                cfs_configuration.add_layer(layer);
            } else {
                // Product layer
                let repo_url = format!(
                    "https://api-gw-service-nmn.local/vcs/cray/{}-config-management.git",
                    layer_yaml["name"].as_str().unwrap()
                );
                let layer = Layer::new(
                    repo_url,
                    // Some(layer_json["product"]["commit"].as_str().unwrap_or_default().to_string()),
                    None,
                    layer_yaml["product"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    layer_yaml["playbook"].as_str().unwrap().to_string(),
                    Some(
                        layer_yaml["product"]["branch"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    ),
                );
                cfs_configuration.add_layer(layer);
            }
        }
        cfs_configuration
    }

    pub async fn create_from_repos(
        gitea_token: &str,
        gitea_base_url: &str,
        shasta_root_cert: &[u8],
        repos: Vec<PathBuf>,
        cfs_configuration_name: &String,
    ) -> Self {
        // Create CFS configuration
        let mut cfs_configuration = configuration::CfsConfigurationRequest::new();
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
            let shasta_commitid_details_resp = gitea::http_client::get_commit_details(
                &api_url,
                // &format!("/cray/{}", repo_name),
                &local_last_commit.id().to_string(),
                gitea_token,
                shasta_root_cert,
            )
            .await;

            // Check sync status between user face and shasta VCS
            let shasta_commitid_details: serde_json::Value = match shasta_commitid_details_resp {
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
            let cfs_layer = configuration::Layer::new(
                clone_url,
                Some(shasta_commitid_details["sha"].as_str().unwrap().to_string()),
                format!(
                    "{}-{}",
                    repo_name.substring(0, repo_name.len()),
                    chrono::offset::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                ),
                String::from("site.yml"),
                None,
            );

            CfsConfigurationRequest::add_layer(&mut cfs_configuration, cfs_layer);
        }

        cfs_configuration
    }
}

/* pub fn add_layer(layer: Layer, mut configuration: CfsConfiguration) -> CfsConfiguration {
    configuration.layers.push(layer);
    configuration
} */

pub mod http_client {

    use std::error::Error;

    use crate::{mesa, shasta::hsm};

    use super::CfsConfigurationRequest;
    use serde_json::Value;

    pub async fn put(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration: &CfsConfigurationRequest,
        configuration_name: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(socks5_env)?;

            // rest client to authenticate
            client_builder.proxy(socks5proxy).build()?
        } else {
            client_builder.build()?
        };

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_name;

        let resp = client
            .put(api_url)
            // .put(format!("{}{}{}", shasta_base_url, "/cfs/v2/configurations/", configuration_name))
            .json(&serde_json::json!({"layers": configuration.layers})) // Encapsulating configuration.layers
            // into an object as required by
            // Shasta API https://apidocs.svc.cscs.ch/paas/cfs/operation/put_configuration/.
            // This seems ugly but this is
            // cleaner than defining
            // configuration.layers as an object
            // with an array inside for no reason
            // other than this call which is
            // encapsulated in this method
            .bearer_auth(shasta_token)
            .send()
            .await?;

        if resp.status().is_success() {
            let response = &resp.text().await?;
            Ok(serde_json::from_str(response)?)
        } else {
            eprintln!("FAIL request: {:#?}", resp);
            let response: String = resp.text().await?;
            eprintln!("FAIL response: {:#?}", response);
            Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_all(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations";

        let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

        let json_response: Value = if resp.status().is_success() {
            serde_json::from_str(&resp.text().await?)?
        } else {
            return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        };

        let configuration_value_vec = json_response.as_array().unwrap().clone();

        log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

        Ok(configuration_value_vec)
    }

    /// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
    /// is built for a specific cluster based on ansible vars used by the CFS session). The reason
    /// for this is because CSCS staff deletes all CFS sessions every now and then...
    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec_opt: Option<&Vec<String>>,
        configuration_name_opt: Option<&String>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut configuration_value_vec = get_all(shasta_token, shasta_base_url, shasta_root_cert)
            .await
            .unwrap();

        log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

        if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
            configuration_value_vec.retain(|configuration_value| {
                hsm_group_name_vec.iter().any(|hsm_group_name| {
                    configuration_value["name"]
                        .as_str()
                        .unwrap()
                        .to_string()
                        .contains(hsm_group_name)
                })
            });
        }

        if let Some(configuration_name) = configuration_name_opt {
            configuration_value_vec.retain(|cfs_configuration| {
                cfs_configuration["name"]
                    .as_str()
                    .unwrap()
                    .eq(configuration_name)
            });
        }

        log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

        configuration_value_vec.sort_by(|a, b| {
            a["lastUpdated"]
                .as_str()
                .unwrap()
                .cmp(b["lastUpdated"].as_str().unwrap())
        });

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            configuration_value_vec = configuration_value_vec[configuration_value_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
                .to_vec();
        }

        log::debug!("CFS configurations:\n{:#?}", configuration_value_vec);

        Ok(configuration_value_vec)
    }

    pub async fn filter(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        mut configuration_value_vec: Vec<Value>,
        hsm_group_name_vec_opt: Option<&Vec<String>>,
        configuration_name_opt: Option<&String>,
        most_recent_opt: Option<bool>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<Value>, Box<dyn Error>> {

        // FILTER BY HSM GROUP NAMES
        if ! hsm_group_name_vec_opt.unwrap().is_empty() {
            if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
                let hsm_group_member_vec = hsm::utils::get_member_vec_from_hsm_name_vec(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    hsm_group_name_vec,
                )
                    .await;

                let cfs_session_vec = mesa::cfs::session::http_client::http_client::get(
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

                /* println!("DEBUG - CFS SESSION");
            for cfs_session in &cfs_session_vec {
                println!(
                    "DEBUG - hsm_group {:?} cfs_configuration {:?}",
                    cfs_session.target.clone().unwrap().groups.unwrap(),
                    cfs_session.configuration
                );
            } */

                let cfs_configuration_name_vec_from_cfs_session = cfs_session_vec
                    .iter()
                    .map(|cfs_session| cfs_session.configuration.clone().unwrap().name.unwrap())
                    .collect::<Vec<_>>();

                let bos_sessiontemplate_vec = mesa::bos::sessiontemplate::http_client::get_all(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                )
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|bos_sessiontemplate| {
                        let boot_set_vec = bos_sessiontemplate
                            .clone()
                            .boot_sets
                            .clone()
                            .unwrap_or_default();

                        let mut boot_set_node_groups_vec = boot_set_vec
                            .iter()
                            .flat_map(|boot_set| boot_set.clone().node_groups.clone().unwrap_or_default());

                        let mut boot_set_node_list_vec = boot_set_vec
                            .iter()
                            .flat_map(|boot_set| boot_set.clone().node_list.clone().unwrap_or_default());

                        boot_set_node_groups_vec.clone().count() > 0
                            && boot_set_node_groups_vec
                            .all(|node_group| hsm_group_name_vec.contains(&node_group))
                            || boot_set_node_list_vec.clone().count() > 0
                            && boot_set_node_list_vec.all(|xname| hsm_group_member_vec.contains(&xname))
                    })
                    .collect::<Vec<_>>();

                /* println!("DEBUG - BOS SESSIONTEMPLATE");
            for bos_sessiontemplate in &bos_sessiontemplate_vec {
                println!(
                    "DEBUG - hsm_group {:?} cfs_configuration {:?}",
                    bos_sessiontemplate
                        .clone()
                        .boot_sets
                        .unwrap()
                        .iter()
                        .flat_map(|boot_set| boot_set.node_groups.clone().unwrap_or_default())
                        .collect::<Vec<_>>(),
                    bos_sessiontemplate.cfs.clone().unwrap().configuration
                );
            } */

                let cfs_configuration_name_from_bos_sessiontemplate = bos_sessiontemplate_vec
                    .iter()
                    .map(|bos_sessiontemplate| {
                        bos_sessiontemplate
                            .cfs
                            .clone()
                            .unwrap()
                            .configuration
                            .clone()
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                let cfs_configuration_name_from_cfs_session_and_bos_settiontemplate = [
                    cfs_configuration_name_vec_from_cfs_session,
                    cfs_configuration_name_from_bos_sessiontemplate,
                ]
                    .concat();

                /* println!(
                "DEBUG - cfs configuration names:\n{:#?}",
                cfs_configuration_name_from_cfs_session_and_bos_settiontemplate
            ); */

                configuration_value_vec.retain(|cfs_configuration| {
                    cfs_configuration_name_from_cfs_session_and_bos_settiontemplate
                        .contains(&cfs_configuration["name"].as_str().unwrap().to_string())
                });

                /* println!(
                "DEBUG - cfs confguration:\n{:#?}",
                cfs_configuration_value_vec
            ); */
            }
        }
        // END FILTER BY HSM GROUP NAME
        if let Some(configuration_name) = configuration_name_opt {
            configuration_value_vec.retain(|cfs_configuration| {
                cfs_configuration["name"]
                    .as_str()
                    .unwrap()
                    .eq(configuration_name)
            });
        }

        configuration_value_vec.sort_by(|a, b| {
            a["lastUpdated"]
                .as_str()
                .unwrap()
                .cmp(b["lastUpdated"].as_str().unwrap())
        });

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            configuration_value_vec = configuration_value_vec[configuration_value_vec
                .len()
                .saturating_sub(*limit_number as usize)..]
                .to_vec();
        }

        // println!("DEBUG - cfs configuration:\n{:#?}", configuration_value_vec.iter().map(|conf| conf["name"].clone()).collect::<Vec<_>>());

        if most_recent_opt.is_some() && most_recent_opt.unwrap() == true {
            configuration_value_vec =
                vec![configuration_value_vec.first().unwrap().clone()].to_vec();
        }

        Ok(configuration_value_vec)
    }

    pub async fn delete(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_id;

        let resp = client
            .delete(api_url)
            .bearer_auth(shasta_token)
            .send()
            .await?;

        if resp.status().is_success() {
            log::debug!("{:#?}", resp);
            Ok(())
        } else {
            log::debug!("{:#?}", resp);
            Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_raw(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<reqwest::Response, reqwest::Error> {
        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(socks5_env)?;

            // rest client to authenticate
            client_builder.proxy(socks5proxy).build()?
        } else {
            client_builder.build()?
        };

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations";

        let network_response_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

        match network_response_rslt {
            Ok(http_response) => http_response.error_for_status(),
            Err(network_error) => Err(network_error),
        }
    }

    pub async fn put_raw(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration: &CfsConfigurationRequest,
        configuration_name: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(socks5_env)?;

            // rest client to authenticate
            client_builder.proxy(socks5proxy).build()?
        } else {
            client_builder.build()?
        };

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/configurations/" + configuration_name;

        let network_response_rslt = client
            .put(api_url)
            .json(&serde_json::json!({"layers": configuration.layers})) // Encapsulating configuration.layers
            .bearer_auth(shasta_token)
            .send()
            .await;

        match network_response_rslt {
            Ok(http_response) => http_response.error_for_status(),
            Err(network_error) => Err(network_error),
        }
    }
}

pub mod utils {

    use comfy_table::Table;
    use serde_json::Value;

    pub fn print_table(cfs_configurations: Vec<Value>) {
        let mut table = Table::new();

        table.set_header(vec!["Name", "Last updated", "Layers"]);

        for cfs_configuration in cfs_configurations {
            let mut layers: String = String::new();

            if cfs_configuration["layers"].as_array().is_some() {
                let layers_json = cfs_configuration["layers"].as_array().unwrap();

                layers = format!(
                    "COMMIT: {} NAME: {}",
                    layers_json[0]["commit"], layers_json[0]["name"]
                );

                for layer in layers_json.iter().skip(1) {
                    layers = format!(
                        "{}\nCOMMIT: {} NAME: {}",
                        layers, layer["commit"], layer["name"]
                    );
                }
            }

            table.add_row(vec![
                cfs_configuration["name"].as_str().unwrap(),
                cfs_configuration["lastUpdated"].as_str().unwrap(),
                &layers,
            ]);
        }

        println!("{table}");
    }
}
