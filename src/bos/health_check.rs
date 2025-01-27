use serde_json::Value;

use crate::common::csm;
use crate::error::Error;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Value, Error> {
    let api_url = shasta_base_url.to_owned() + "/bos/v2/healthz";

    let response = csm::process_get_http_request(shasta_token, api_url, shasta_root_cert).await;
    response
}
