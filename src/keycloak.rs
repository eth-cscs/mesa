pub mod http_client {
    use crate::error::Error;

    pub async fn create_role(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        group_name_opt: Option<&String>,
        role_name: &str,
        role_description: &str,
    ) -> Result<(), Error> {
        let realm = "shasta";
        let client_uuid = "838a32c1-8b1c-408f-9c94-2a431cb8e713";

        let client_uuid = "";
        log::info!("Add Role {}", role_name);
        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url: String =
            format!("{shasta_base_url}/keycloak/admin/realms/{realm}/clients/{client_uuid}/roles");

        let role = serde_json::json!({
            "name": role_name,
            "description": role_description
        });

        client
            .post(api_url)
            .header("Authorization", format!("Bearer {}", shasta_token))
            .json(&role) // make sure this is not a string!
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        // TODO Parse the output!!!
        // TODO add some debugging output

        Ok(())
    }
}
