use serde_json::Value;

/// Get components data.
/// Currently, CSM will throw an error if many xnames are sent in the request, therefore, this
/// method will paralelize multiple calls, each with a batch of xnames
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_groups_node_list: &[String],
) -> Result<Vec<Value>, reqwest::Error> {
    let chunk_size = 30;

    let mut component_vec = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    for sub_node_list in hsm_groups_node_list.chunks(chunk_size) {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shata_root_cert_vec = shasta_root_cert.to_vec();

        let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

        tasks.spawn(async move {
            crate::cfs::component::shasta::http_client::get_multiple_components(
                &shasta_token_string,
                &shasta_base_url_string,
                &shata_root_cert_vec,
                Some(&hsm_subgroup_nodes_string),
                None,
            )
            .await
            .unwrap()
        });
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(node_status_vec) = message {
            component_vec = [component_vec, node_status_vec].concat();
        }
    }

    Ok(component_vec)
}
