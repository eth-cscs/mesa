//! Fetch Kubernetes service-account secrets from Vault — the supported way to obtain CSM cluster credentials off-cluster.

/// Vault HTTP helpers: OIDC login + secret fetching for CSM and VCS
/// credentials stored under `secret/manta/data/<site>`.
pub mod http_client {

  use crate::error::Error;
  use serde_json::{Value, json};

  /// Exchange a Shasta (Keycloak) JWT for a Vault token via the OIDC
  /// JWT auth backend. The Vault role is hard-coded to `manta`.
  pub async fn auth_oidc_jwt(
    vault_base_url: &str,
    // vault_role_id: &str,
    shasta_token: &str,
    site_name: &str,
  ) -> Result<String, Error> {
    let role = "manta";

    let client = reqwest::Client::builder()
      .connect_timeout(crate::common::http::HTTP_CONNECT_TIMEOUT)
      .build()?;

    let api_url =
      format!("{vault_base_url}/v1/auth/jwt-manta-{site_name}/login");

    log::debug!("Accessing/login to {api_url}");

    let request_payload = json!({
            "jwt": shasta_token,
            "role": role});

    let resp = client
      .post(api_url)
      .header("X-Vault-Request", "true")
      .json(&request_payload)
      .send()
      .await?;

    match resp.error_for_status() {
      Ok(resp) => {
        let resp_value = resp.json::<Value>().await?;
        resp_value
          .get("auth")
          .and_then(|auth| auth.get("client_token"))
          .and_then(Value::as_str)
          .map(String::from)
          .ok_or_else(|| {
            Error::Message("JWT auth token not valid".to_string())
          })
      }
      Err(e) => Err(Error::NetError(e)),
    }
  }

  /// Low-level Vault read: `GET <vault_base_url><secret_path>` with the
  /// supplied Vault token, returning the secret's `.data` payload.
  pub async fn fetch_secret(
    vault_auth_token: &str,
    vault_base_url: &str,
    secret_path: &str,
  ) -> Result<Value, Error> {
    let client = reqwest::Client::builder()
      .connect_timeout(crate::common::http::HTTP_CONNECT_TIMEOUT)
      .build()?;

    let api_url = vault_base_url.to_owned() + secret_path;

    log::debug!("Vault url to fetch VCS secrets is '{api_url}'");

    let resp = client
      .get(api_url)
      .header("X-Vault-Token", vault_auth_token)
      .send()
      .await?;

    match resp.error_for_status() {
      Ok(resp) => {
        let secret_value: Value = resp.json().await?;
        Ok(secret_value["data"].clone())
      }
      Err(e) => Err(Error::NetError(e)),
    }
  }

  /// Fetch the Kubernetes API URL, token, and CA cert from
  /// `secret/manta/data/<site>/k8s` — the credentials csm-rs uses to
  /// read the in-cluster `cray-product-catalog` `ConfigMap` and to attach
  /// node consoles.
  pub async fn fetch_shasta_k8s_secrets_from_vault(
    vault_base_url: &str,
    // vault_role_id: &str,
    shasta_token: &str,
    // secret_path: &str,
    site_name: &str,
  ) -> Result<Value, Error> {
    log::debug!("Fetching k8s secrets from vault");
    let vault_token =
      auth_oidc_jwt(vault_base_url, shasta_token, site_name).await?;

    let vault_secret_path = format!("manta/data/{site_name}");

    fetch_secret(
      &vault_token,
      vault_base_url,
      &format!("/v1/{vault_secret_path}/k8s"),
    )
    .await
    .map(|secret| secret["data"].clone())
  }
}
