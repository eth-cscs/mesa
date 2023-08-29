pub mod http_client {

    use std::error::Error;

    use serde_json::Value;

    /// Fetch IMS image ref --> https://apidocs.svc.cscs.ch/paas/ims/operation/get_v3_image/
    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        username_opt: Option<&str>,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let client;

        let client_builder = reqwest::Client::builder().danger_accept_invalid_certs(true);

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

        let api_url = shasta_base_url.to_owned() + "/ims/v3/public-keys";

        let resp = client
            .get(api_url)
            // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
            .bearer_auth(shasta_token)
            .send()
            .await?;

        let json_response: Value = if resp.status().is_success() {
            serde_json::from_str(&resp.text().await?)?
        } else {
            eprintln!("FAIL request: {:#?}", resp);
            let response: String = resp.text().await?;
            eprintln!("FAIL response: {:#?}", response);
            return Err(response.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        };

        let mut public_key_value_list: Vec<Value> = json_response.as_array().unwrap().to_vec();

        public_key_value_list = if let Some(username) = username_opt {
            public_key_value_list
                .retain(|ssh_key_value| ssh_key_value["name"].as_str().unwrap().eq(username));

            public_key_value_list
        } else {
            json_response.as_array().unwrap().to_vec()
        };

        Ok(public_key_value_list.to_vec())
    }

    pub async fn get_single(
        shasta_token: &str,
        shasta_base_url: &str,
        username_opt: Option<&str>,
    ) -> Option<Value> {
        if let Ok(public_key_value_list) = get(shasta_token, shasta_base_url, username_opt).await {
            if public_key_value_list.len() == 1 {
                return public_key_value_list.first().cloned()
            };
        }

        None
    }
}
