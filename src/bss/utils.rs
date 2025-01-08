use std::collections::HashMap;

use super::types::BootParameters;

// Assumes s3 path looks like:
// - s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/kernel
// - craycps-s3:s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs:3dfae8d1fa3bb2bfb18152b4f9940ad0-667:dvs:api-gw-service-nmn.local:300:nmn0,hsn0:0
// - url=s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs,etag=3dfae8d1fa3bb2bfb18152b4f9940ad0-667 bos_update_frequency=4h
pub fn get_image_id_from_s3_path(s3_path: &str) -> Option<&str> {
    s3_path.split("/").skip(3).next()
}

pub fn convert_kernel_params_to_map(kernel_params: &str) -> HashMap<String, String> {
    kernel_params
        .split_whitespace()
        .map(|kernel_param| {
            let (key_str, value_str) = kernel_param.split_once('=').unwrap_or((kernel_param, ""));

            let key = key_str.to_string();
            let value = value_str.to_string();

            (key, value)
        })
        .collect()
}

pub fn find_boot_params_related_to_node(
    node_boot_params_list: &[BootParameters],
    node: &String,
) -> Option<BootParameters> {
    node_boot_params_list
        .iter()
        .find(|node_boot_param| node_boot_param.hosts.iter().any(|host| host.eq(node)))
        .cloned()
}

/* /// Get Image ID from kernel field
#[deprecated(
    since = "1.26.6",
    note = "Please convert from serde_json::Value to struct BootParameters use function `BootParameters::get_boot_image` instead"
)]
pub fn get_image_id(node_boot_params: &Value) -> String {
    serde_json::from_value::<BootParameters>(node_boot_params.clone())
        .unwrap()
        .get_boot_image()
} */
