//! Keycloak / OIDC bearer-token acquisition for Shasta.

use serde_json::Value;

use std::collections::HashMap;

use crate::error::Error;

/// Validate a CSM bearer token by issuing `GET /cfs/healthz` and
/// checking the response status.
pub async fn validate_api_token(
  shasta_base_url: &str,
  shasta_token: &str,
  shasta_root_cert: &[u8],
) -> Result<(), Error> {
  let client = crate::common::http::build_client(shasta_root_cert)?;

  let api_url = shasta_base_url.to_owned() + "/cfs/healthz";

  log::debug!("Validate CSM token against {api_url}");

  let resp_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

  match resp_rslt {
    Ok(resp) => Ok(resp.error_for_status().map(|_| ())?),
    Err(error) => Err(Error::Message(format!("Token is not valid: {error}"))),
  }
}

/// Exchange Keycloak username/password credentials for a CSM bearer
/// token via the `password` grant.
pub async fn get_token_from_shasta_endpoint(
  keycloak_base_url: &str,
  shasta_root_cert: &[u8],
  username: &str,
  password: &str,
) -> Result<String, Error> {
  let mut params = HashMap::new();
  params.insert("grant_type", "password");
  params.insert("client_id", "shasta");
  params.insert("username", username);
  params.insert("password", password);

  let client = crate::common::http::build_client(shasta_root_cert)?;

  let api_url = format!(
    "{keycloak_base_url}/realms/shasta/protocol/openid-connect/token"
  );

  log::debug!("Request to fetch authentication token: {api_url}");

  client
    .post(api_url)
    .form(&params)
    .send()
    .await?
    .error_for_status()?
    .json::<Value>()
    .await?
    .get("access_token")
    .and_then(Value::as_str)
    .map(str::to_string)
    .ok_or_else(|| {
      Error::Message(
        "Keycloak token response is missing 'access_token'".to_string(),
      )
    })
}
