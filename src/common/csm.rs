use serde_json::Value;

use crate::error::Error;

/// This function will create an http client and call CSM API endpoint "shasta_url"
/// Returns the same payload received from CSM API
pub async fn get_csm_api_url(
    shasta_token: &str,
    shasta_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Value, Error> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    // Set SOCKS5 proxy if needed. User can set SOCKS5 proxy using the 'SOCKS5' environment
    // variable with the actual proxy URI
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }   

    let api_url = shasta_url.to_owned();

    // Call to CSM API
    let response = client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    // Error handlelilng. Check for network errors and/or errors from the CSM API processing the
    // request
    if response.status().is_success() {
        response
            .json()
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
