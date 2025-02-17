use reqwest::Url;
use serde_json::Value;

use crate::error::Error;

pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_vec: &[String],
) -> Result<Vec<Value>, Error> {
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

    let url_params: Vec<_> = xname_vec.iter().map(|xname| ("id", xname)).collect();

    let api_url = Url::parse_with_params(
        &format!("{}/smd/hsm/v2/State/Components", shasta_base_url),
        &url_params,
    )
    .unwrap();

    let response = client
        .get(api_url.clone())
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))
            .unwrap()["Components"]
            .as_array()
            .unwrap_or(&Vec::new())
            .clone())
    } else {
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::CsmError(payload))
    }
}

/// Fetches nodes/compnents details using HSM v2 ref --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doComponentsGet/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_vec: &[String],
) -> Result<Vec<Value>, Error> {
    let chunk_size = 30;

    let mut hsm_component_status_vec: Vec<Value> = Vec::new();

    let mut tasks = tokio::task::JoinSet::new();

    for sub_node_list in xname_vec.chunks(chunk_size) {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        // let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

        let node_vec = sub_node_list.to_vec();

        tasks.spawn(async move {
            get_raw(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                &node_vec,
            )
            .await
        });
    }

    while let Some(message) = tasks.join_next().await {
        match message.unwrap() {
            Ok(mut node_status_vec) => {
                hsm_component_status_vec.append(&mut node_status_vec);
            }
            Err(error) => {
                log::error!("Error: {:?}", error);
            }
        }
        /* if let Ok(mut node_status_vec) = message {
            hsm_component_status_vec.append(&mut node_status_vec);
        } */
    }

    Ok(hsm_component_status_vec)
}
