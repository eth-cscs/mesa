use serde_json::Value;

use crate::bos::template::mesa::r#struct::response_payload::BosSessionTemplate;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id: Option<&String>,
) -> Result<Vec<BosSessionTemplate>, reqwest::Error> {
    let bos_sessiontemplate_response_value = crate::bos::template::shasta::http_client::get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        bos_session_template_id,
    )
    .await;

    let bos_sessiontemplate_response_value: Value = match bos_sessiontemplate_response_value {
        Ok(bos_sessiontemplate_value) => bos_sessiontemplate_value.json().await.unwrap(),
        Err(error) => return Err(error),
    };

    let mut bos_sessiontemplate_vec = Vec::new();

    if let Some(bos_sessiontemplate_value_vec) = bos_sessiontemplate_response_value.as_array() {
        for bos_sessiontemplate_value in bos_sessiontemplate_value_vec {
            bos_sessiontemplate_vec.push(BosSessionTemplate::from_csm_api_json(
                bos_sessiontemplate_value.clone(),
            ));
        }
    } else {
        bos_sessiontemplate_vec.push(BosSessionTemplate::from_csm_api_json(
            bos_sessiontemplate_response_value,
        ));
    }

    Ok(bos_sessiontemplate_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<BosSessionTemplate>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}
