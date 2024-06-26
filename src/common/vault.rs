pub mod http_client {

    use serde_json::{json, Value};

    pub async fn auth(vault_base_url: &str, vault_role_id: &str) -> Result<String, reqwest::Error> {
        // rest client create new cfs sessions
        let client = reqwest::Client::builder().build()?;

        let api_url = vault_base_url.to_owned() + "/v1/auth/approle/login";

        log::debug!("Accessing/login to {}", api_url);

        Ok(client
            .post(api_url.clone())
            // .post(format!("{}{}", vault_base_url, "/v1/auth/approle/login"))
            .json(&json!({ "role_id": vault_role_id }))
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?
            .pointer("/auth/client_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string())
    }

    pub async fn fetch_secret(
        auth_token: &str,
        vault_base_url: &str,
        vault_secret_path: &str,
    ) -> Result<Value, reqwest::Error> {
        // rest client create new cfs sessions
        let client = reqwest::Client::builder().build()?;

        let api_url = vault_base_url.to_owned() + vault_secret_path;

        log::debug!("Vault url to fetch VCS secrets is '{}'", api_url);

        Ok(client
            .get(api_url)
            .header("X-Vault-Token", auth_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?["data"]
            .clone())
    }

    pub async fn fetch_shasta_vcs_token(
        vault_base_url: &str,
        vault_secrets_path: &str,
        vault_role_id: &str,
    ) -> Result<String, reqwest::Error> {
        let vault_token_resp = auth(vault_base_url, vault_role_id).await;

        match vault_token_resp {
            Ok(vault_token) => {
                let vault_secret = fetch_secret(
                    &vault_token,
                    vault_base_url,
                    &format!("/v1/{}/vcs", vault_secrets_path),
                )
                .await?; // this works for hashicorp-vault for fulen may need /v1/secret/data/shasta/vcs
                Ok(String::from(vault_secret["token"].as_str().unwrap())) // this works for vault v1.12.0 for older versions may need vault_secret["data"]["token"]
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    pub async fn fetch_shasta_k8s_secrets(
        vault_base_url: &str,
        vault_secret_path: &str,
        vault_role_id: &str,
    ) -> Value {
        let vault_token_resp = auth(vault_base_url, vault_role_id).await;

        match vault_token_resp {
            Ok(vault_token) => {
                let vault_secret = fetch_secret(
                    &vault_token,
                    vault_base_url,
                    &format!("/v1/{}/k8s", vault_secret_path),
                )
                .await
                .unwrap(); // this works for hashicorp-vault for fulen may need /v1/secret/data/shasta/k8s

                serde_json::from_str::<Value>(vault_secret["value"].as_str().unwrap()).unwrap()
                // this works for vault v1.12.0 for older versions may need vault_secret["data"]["value"]
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
