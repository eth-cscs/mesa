use serde_json::Value;

use crate::error::Error;

use super::r#struct::CfsComponent;

pub async fn get_multiple_components(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    components_ids: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<CfsComponent>, Error> {
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

    let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

    let response = client
        .get(api_url)
        .query(&[("ids", components_ids), ("status", status)])
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        response
            .json::<Vec<CfsComponent>>()
            .await
            .map_err(|error| Error::NetError(error))
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}

/// Get components data.
/// Currently, CSM will throw an error if many xnames are sent in the request, therefore, this
/// method will paralelize multiple calls, each with a batch of xnames
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_groups_node_list: &[String],
) -> Result<Vec<CfsComponent>, Error> {
    let chunk_size = 30;

    let mut component_vec = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    for sub_node_list in hsm_groups_node_list.chunks(chunk_size) {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

        tasks.spawn(async move {
            get_multiple_components(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                Some(&hsm_subgroup_nodes_string),
                None,
            )
            .await
            .unwrap()
        });
    }

    while let Some(message) = tasks.join_next().await {
        if let Ok(mut node_status_vec) = message {
            component_vec.append(&mut node_status_vec);
        }
    }

    Ok(component_vec)
}
