use regex::Regex;
use serde_json::Value;

use crate::{bss, cfs, hsm};

use super::r#struct::NodeDetails;

pub fn validate_xname_format(xname: &str) -> bool {
    let xname_re = Regex::new(r"^x\d{4}c[0-7]s([0-9]|[1-5][0-9]|6[0-4])b[0-1]n[0-7]$").unwrap();

    xname_re.is_match(xname)
}

/// Validates a list of xnames.
/// Checks xnames strings are valid
/// If hsm_group_name if provided, then checks all xnames belongs to that hsm_group
pub async fn validate_xnames(
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

/// Get components data.
/// Currently, CSM will throw an error if many xnames are sent in the request, therefore, this
/// method will paralelize multiple calls, each with a batch of xnames
pub async fn get_node_details(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_groups_node_list: Vec<String>,
) -> Vec<NodeDetails> {
    // Get CFS component status
    let components_status = cfs::component::mesa::http_client::get_multiple(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &hsm_groups_node_list,
    )
    .await
    .unwrap();

    // Get boot params to get the boot image id for each node
    let node_boot_params_vec = crate::bss::bootparameters::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &hsm_groups_node_list,
    )
    .await
    .unwrap();

    // Get HSM component status (needed to get NIDS)
    let node_hsm_info_resp = hsm::component_status::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &hsm_groups_node_list,
    )
    .await
    .unwrap();

    // Get CFS sessions
    let cfs_session_vec = crate::cfs::session::mesa::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap();

    // Get BOS session template
    let _ = crate::bos::template::mesa::http_client::get_all(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
    )
    .await
    .unwrap();

    // match node with bot_sessiontemplate and put them in a list
    let mut node_details_vec = Vec::new();

    for node in &hsm_groups_node_list {
        // let mut node_details = Vec::new();

        // find component details
        let component_details_opt = components_status
            .iter()
            .find(|component_status| component_status.id.eq(node));

        let component_details = if let Some(component_details) = component_details_opt {
            component_details
        } else {
            eprintln!(
                "ERROR - CFS component details for node {}.\nReason:\n{:#?}",
                node, component_details_opt
            );
            std::process::exit(1);
        };

        let desired_configuration = &component_details.desired_config;
        let configuration_status = &component_details.configuration_status;
        let enabled = component_details.enabled;
        let error_count = component_details.error_count;

        // get power status
        let node_hsm_info = node_hsm_info_resp
            // ["Components"]
            // .as_array()
            // .unwrap()
            .iter()
            .find(|&component| component["ID"].as_str().unwrap().eq(node))
            .unwrap();

        let node_power_status = node_hsm_info["State"]
            .as_str()
            .unwrap()
            .to_string()
            .to_uppercase();

        let node_nid = format!(
            "nid{:0>6}",
            node_hsm_info["NID"].as_u64().unwrap().to_string()
        );

        // get node boot params (these are the boot params of the nodes with the image the node
        // boot with). the image in the bos sessiontemplate may be different i don't know why. need
        // to investigate
        let (kernel_image_path_in_boot_params, kernel_params): (String, String) =
            if let Some(node_boot_params) =
                bss::bootparameters::utils::find_boot_params_related_to_node(
                    &node_boot_params_vec,
                    node,
                )
            {
                (node_boot_params.get_boot_image(), node_boot_params.params)
            } else {
                eprintln!("BSS boot parameters for node {} - NOT FOUND", node);
                ("Not found".to_string(), "Not found".to_string())
            };

        // Get CFS configuration related to image id
        let cfs_session_related_to_image_id_opt =
            cfs::session::mesa::utils::find_cfs_session_related_to_image_id(
                &cfs_session_vec,
                &kernel_image_path_in_boot_params,
            );

        let cfs_configuration_boot = if let Some(cfs_session_related_to_image_id) =
            cfs_session_related_to_image_id_opt
        {
            cfs::session::mesa::utils::get_cfs_configuration_name(&cfs_session_related_to_image_id)
                .unwrap()
        } else {
            log::warn!(
                "No configuration found for node {} related to image id {}",
                node,
                kernel_image_path_in_boot_params
            );
            "Not found".to_string()
        };

        let node_details = NodeDetails {
            xname: node.to_string(),
            nid: node_nid,
            power_status: node_power_status,
            desired_configuration: desired_configuration.to_owned(),
            configuration_status: configuration_status.to_owned(),
            enabled: enabled.to_string(),
            error_count: error_count.to_string(),
            boot_image_id: kernel_image_path_in_boot_params,
            boot_configuration: cfs_configuration_boot,
            kernel_params,
        };

        node_details_vec.push(node_details);
    }

    node_details_vec
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
