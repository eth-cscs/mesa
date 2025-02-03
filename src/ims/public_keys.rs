pub mod http_client {

    pub mod v3 {
        use serde_json::Value;

        use crate::error::Error;

        /// Get one user public key in IMS is can find
        /// Returns None if public key not found or multiple fould
        pub async fn get_single(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            username_opt: &str,
        ) -> Option<Value> {
            if let Ok(public_key_value_list) = get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                Some(username_opt),
            )
            .await
            {
                if public_key_value_list.len() == 1 {
                    return public_key_value_list.first().cloned();
                };
            }

            None
        }

        /// Fetch IMS image ref --> https://apidocs.svc.cscs.ch/paas/ims/operation/get_v3_image/
        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            username_opt: Option<&str>,
        ) -> Result<Vec<Value>, Error> {
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

            let api_url = shasta_base_url.to_owned() + "/ims/v3/public-keys";

            let json_response: Value = client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(Error::NetError)?
                .json()
                .await
                .map_err(Error::NetError)?;

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
    }
}
