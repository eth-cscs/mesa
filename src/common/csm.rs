use serde_json::Value;

use crate::error::Error;

/// Create GET HTTP request and returns its response
/// This function will create an http client and call CSM API endpoint "api_url"
/// Returns the same payload received from CSM API
pub async fn process_get_http_request(
    shasta_token: &str,
    api_url: String,
    shasta_root_cert: &[u8],
) -> Result<Value, Error> {
    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    // Set SOCKS5 proxy if needed. User can set SOCKS5 proxy using the 'SOCKS5' environment
    // variable with the actual proxy URI
    let client = if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    // Call to CSM API
    let response = client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|e| Error::NetError(e))?; // Map network errors

    // Error handleling. Check for errors from the CSM API processing the request
    match response.status().is_success() {
        true => response.json().await.map_err(|e| Error::NetError(e)), // Map error during marshalling
        false => {
            let e: Value = response.json().await.map_err(|e| Error::NetError(e))?; // Map error during

            Err(Error::CsmError(e))
        }
    }
}
