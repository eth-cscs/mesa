use std::{collections::HashMap, sync::Arc, time::Instant};

use regex::Regex;
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::{bss, cfs, error::Error, hsm};

use super::types::NodeDetails;

/// Check if input is a NID
pub fn validate_nid_format_regex(node_vec: Vec<String>, regex: Regex) -> bool {
    node_vec.iter().all(|nid| regex.is_match(nid))
}

/// Check if input is a NID
pub fn validate_nid_format_vec(node_vec: Vec<String>) -> bool {
    node_vec.iter().all(|nid| validate_nid_format(nid))
}

/// Check if input is a NID
pub fn validate_nid_format(nid: &str) -> bool {
    nid.to_lowercase().starts_with("nid")
        && nid.len() == 9
        && nid
            .strip_prefix("nid")
            .is_some_and(|nid_number| nid_number.chars().all(char::is_numeric))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format_regex(node_vec: Vec<String>, regex: Regex) -> bool {
    node_vec.iter().all(|nid| regex.is_match(nid))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format_vec(node_vec: Vec<String>) -> bool {
    node_vec.iter().all(|nid| validate_xname_format(nid))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format(xname: &str) -> bool {
    let xname_re = Regex::new(r"^x\d{4}c[0-7]s([0-9]|[1-5][0-9]|6[0-4])b[0-1]n[0-7]$").unwrap();

    xname_re.is_match(xname)
}

/// Validates a list of xnames.
/// Checks xnames strings are valid
/// If hsm_group_name_opt provided, then checks all xnames belongs to that hsm_group
pub async fn validate_xnames_format_and_membership_agaisnt_single_hsm(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xnames: &[&str],
    hsm_group_name_opt: Option<&String>,
) -> bool {
    let hsm_group_members: Vec<String> = if let Some(hsm_group_name) = hsm_group_name_opt {
        hsm::group::utils::get_member_vec_from_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name,
        )
        .await
    } else {
        Vec::new()
    };

    if xnames.iter().any(|&xname| {
        !validate_xname_format(xname)
            || (!hsm_group_members.is_empty() && !hsm_group_members.contains(&xname.to_string()))
    }) {
        return false;
    }

    true
}

/// Validates a list of xnames.
/// Checks xnames strings are valid
/// If hsm_group_name_vec_opt provided, then checks all xnames belongs to those hsm_groups
pub async fn validate_xnames_format_and_membership_agaisnt_multiple_hsm(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xnames: &[&str],
    hsm_group_name_vec_opt: Option<Vec<String>>,
) -> bool {
    let hsm_group_members: Vec<String> =
        if let Some(hsm_group_name) = hsm_group_name_vec_opt.clone() {
            hsm::group::utils::get_member_vec_from_hsm_name_vec(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name,
            )
            .await
            .unwrap()
        } else {
            Vec::new()
        };

    if xnames.iter().any(|&xname| {
        !validate_xname_format(xname)
            || (!hsm_group_members.is_empty() && !hsm_group_members.contains(&xname.to_string()))
    }) {
        return false;
    }

    true
}

/// Get components data.
/// Currently, CSM will throw an error if many xnames are sent in the request, therefore, this
/// method will paralelize multiple calls, each with a batch of xnames
pub async fn get_node_details(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_list: Vec<String>,
) -> Result<Vec<NodeDetails>, Error> {
    let start = Instant::now();

    let (
        components_status_rslt,
        node_boot_params_vec_rslt,
        node_hsm_info_rslt,
        cfs_session_vec_rslt,
    ) = tokio::join!(
        // Get CFS component status
        cfs::component::http_client::v2::get_multiple(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &xname_list,
        ),
        // Get boot params to get the boot image id for each node
        bss::http_client::get_multiple(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &xname_list,
        ),
        // Get HSM component status (needed to get NIDS)
        hsm::component_status::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &xname_list,
        ),
        // Get CFS sessions
        cfs::session::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
            None,
            None,
            None,
            Some(true),
        )
    );

    // ------------------------------------------------------------------------
    // Get and collect HSM members
    let mut node_details_map = HashMap::new();
    let mut tasks = tokio::task::JoinSet::new();

    let sem = Arc::new(Semaphore::new(10)); // CSM 1.3.1 higher number of concurrent tasks won't

    for xname in xname_list {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        let components_status = components_status_rslt.as_ref().unwrap();

        // find component details
        let component_details_opt = components_status
            .iter()
            .find(|component_status| component_status.id.as_ref().unwrap().eq(&xname));

        // FIXME: fix this by converting 'compoennt_details_opt' into a Result, with
        // backend-dispatcher::Error and resolve the value using '?'
        let component_details = if let Some(component_details) = component_details_opt {
            component_details
        } else {
            return Err(Error::Message(format!(
                "ERROR - CFS component details for node {}.\nReason:\n{:#?}",
                xname, component_details_opt
            )));
        };

        let desired_configuration = &component_details.desired_config;
        let configuration_status = &component_details.configuration_status;
        let enabled = component_details.enabled;
        let error_count = component_details.error_count.clone();

        // Get node HSM details
        let node_hsm_info_value = node_hsm_info_rslt
            .as_ref()
            .unwrap()
            .iter()
            .find(|component| component["ID"].as_str().unwrap().eq(&xname))
            .unwrap();

        // Gget power status
        let node_power_status = node_hsm_info_value["State"]
            .as_str()
            .unwrap()
            .to_string()
            .to_uppercase();

        // Calculate NID
        let node_nid = format!(
            "nid{:0>6}",
            node_hsm_info_value["NID"].as_u64().unwrap().to_string()
        );

        // get node boot params (these are the boot params of the nodes with the image the node
        // boot with). the image in the bos sessiontemplate may be different i don't know why. need
        // to investigate
        let (image_id_in_kernel_params, kernel_params): (String, String) =
            if let Some(node_boot_params) = bss::utils::find_boot_params_related_to_node(
                &node_boot_params_vec_rslt.as_ref().unwrap(),
                &xname,
            ) {
                (node_boot_params.get_boot_image(), node_boot_params.params)
            } else {
                eprintln!("BSS boot parameters for node '{}' - NOT FOUND", xname);
                ("Not found".to_string(), "Not found".to_string())
            };

        // Get CFS configuration related to image id
        let cfs_session_related_to_image_id_opt =
            cfs::session::utils::find_cfs_session_related_to_image_id(
                &cfs_session_vec_rslt.as_ref().unwrap(),
                &image_id_in_kernel_params,
            );

        let cfs_configuration_boot =
            if let Some(cfs_session_related_to_image_id) = cfs_session_related_to_image_id_opt {
                cfs::session::utils::get_cfs_configuration_name(&cfs_session_related_to_image_id)
                    .unwrap()
            } else {
                log::warn!(
                    "No configuration found for node '{}' related to image id '{}'",
                    xname,
                    image_id_in_kernel_params
                );
                "Not found".to_string()
            };

        node_details_map
            .entry(xname.clone())
            .and_modify(|node_details: &mut NodeDetails| {
                node_details.xname = xname.clone();
                node_details.nid = node_nid.clone();
                node_details.hsm = "".to_string();
                node_details.power_status = node_power_status.clone();
                node_details.desired_configuration = desired_configuration.clone().unwrap();
                node_details.configuration_status = configuration_status.clone().unwrap();
                node_details.enabled = enabled.unwrap().to_string();
                node_details.error_count = error_count.unwrap().to_string();
                node_details.boot_image_id = image_id_in_kernel_params.clone();
                node_details.boot_configuration = cfs_configuration_boot.clone();
                node_details.kernel_params = kernel_params.clone();
            })
            .or_insert(NodeDetails {
                xname: xname.clone(),
                nid: node_nid,
                hsm: "".to_string(),
                power_status: node_power_status,
                desired_configuration: desired_configuration.clone().unwrap(),
                configuration_status: configuration_status.clone().unwrap(),
                enabled: enabled.unwrap().to_string(),
                error_count: error_count.unwrap().to_string(),
                boot_image_id: image_id_in_kernel_params,
                boot_configuration: cfs_configuration_boot,
                kernel_params,
            });

        let permit = Arc::clone(&sem).acquire_owned().await;

        tasks.spawn(async move {
            let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

            hsm::memberships::http_client::get_xname(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                &xname,
            )
            .await
            .expect(&format!(
                "ERROR - could not get node '{}' membership from HSM",
                xname
            ))
        });
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(node_membership) = message {
            let node_details = NodeDetails {
                xname: "".to_string(),
                nid: "".to_string(),
                hsm: node_membership.group_labels.join(", "),
                power_status: "".to_string(),
                desired_configuration: "".to_string(),
                configuration_status: "".to_string(),
                enabled: "".to_string(),
                error_count: "".to_string(),
                boot_image_id: "".to_string(),
                boot_configuration: "".to_string(),
                kernel_params: "".to_string(),
            };

            node_details_map
                .entry(node_membership.id.clone())
                .and_modify(|node_details: &mut NodeDetails| {
                    node_details.hsm = node_membership.group_labels.join(", ")
                })
                .or_insert(node_details);
        }
    }

    let duration = start.elapsed();
    log::info!("Time elapsed to get node details is: {:?}", duration);
    // ------------------------------------------------------------------------

    Ok(node_details_map.into_values().collect())
}

pub fn nodes_to_string_format_one_line(nodes: Option<&Vec<Value>>) -> String {
    if let Some(nodes_content) = nodes {
        nodes_to_string_format_discrete_columns(nodes, nodes_content.len() + 1)
    } else {
        "".to_string()
    }
}

pub fn nodes_to_string_format_discrete_columns(
    nodes: Option<&Vec<Value>>,
    num_columns: usize,
) -> String {
    let mut members: String;

    match nodes {
        Some(nodes) if !nodes.is_empty() => {
            members = nodes[0].as_str().unwrap().to_string(); // take first element

            for (i, _) in nodes.iter().enumerate().skip(1) {
                // iterate for the rest of the list
                if i % num_columns == 0 {
                    // breaking the cell content into multiple lines (only 2 xnames per line)

                    members.push_str(",\n");
                } else {
                    members.push(',');
                }

                members.push_str(nodes[i].as_str().unwrap());
            }
        }
        _ => members = "".to_string(),
    }

    members
}

pub fn string_vec_to_multi_line_string(nodes: Option<&Vec<String>>, num_columns: usize) -> String {
    let mut members: String;

    match nodes {
        Some(nodes) if !nodes.is_empty() => {
            members = nodes.first().unwrap().to_string(); // take first element

            for (i, _) in nodes.iter().enumerate().skip(1) {
                // iterate for the rest of the list
                if i % num_columns == 0 {
                    // breaking the cell content into multiple lines (only 2 xnames per line)

                    members.push_str(",\n");
                } else {
                    members.push(',');
                }

                members.push_str(&nodes[i]);
            }
        }
        _ => members = "".to_string(),
    }

    members
}
