use core::time;
use serde_json::Value;
use std::io::Write;

use crate::capmc::http_client::{node_power_off, node_power_on, node_power_status};

pub async fn wait_nodes_to_power_on(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_vec: Vec<String>,
    reason: Option<String>,
) -> Result<Value, reqwest::Error> {
    let mut node_status_value: Value =
        node_power_status::post(shasta_token, shasta_base_url, shasta_root_cert, &xname_vec)
            .await
            .unwrap();

    let mut node_off_vec: Vec<String> = node_status_value["off"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|xname: &Value| xname.as_str().unwrap().to_string())
        .collect();

    // Check all nodes are OFF
    let mut i = 0;
    let max = 60;
    let delay_secs = 3;
    while i <= max && !node_off_vec.is_empty() {
        let _ = node_power_on::post(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            xname_vec.clone(),
            reason.clone(),
        )
        .await;

        tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

        node_status_value =
            node_power_status::post(shasta_token, shasta_base_url, shasta_root_cert, &xname_vec)
                .await
                .unwrap();

        node_off_vec = node_status_value["off"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|xname: &Value| xname.as_str().unwrap().to_string())
            .collect();

        print!(
            "\rWaiting nodes to power on. Trying again in {} seconds. Attempt {} of {}.",
            delay_secs,
            i + 1,
            max
        );
        std::io::stdout().flush().unwrap();

        i += 1;
    }

    Ok(node_status_value)
}

pub async fn wait_nodes_to_power_off(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_vec: Vec<String>,
    reason_opt: Option<String>,
    force: bool,
) -> Result<Value, reqwest::Error> {
    let mut node_off_vec: Vec<String> = Vec::new();
    let mut node_status_value: Value = serde_json::Value::Null;

    // Check all nodes are OFF
    let mut i = 0;
    let max = 60;
    let delay_secs = 3;
    while i <= max && xname_vec.iter().any(|xname| !node_off_vec.contains(xname)) {
        let _ = node_power_off::post(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            xname_vec.clone(),
            reason_opt.clone(),
            force,
        )
        .await;

        tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

        node_status_value =
            node_power_status::post(shasta_token, shasta_base_url, shasta_root_cert, &xname_vec)
                .await
                .unwrap();

        node_off_vec = node_status_value["off"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|xname: &Value| xname.as_str().unwrap().to_string())
            .collect();

        print!(
            "\rWaiting nodes to power off. Trying again in {} seconds. Attempt {} of {}.",
            delay_secs,
            i + 1,
            max
        );
        std::io::stdout().flush().unwrap();

        i += 1;
    }

    Ok(node_status_value)
}
