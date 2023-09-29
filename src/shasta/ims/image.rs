pub mod http_client {

    use std::error::Error;

    use serde_json::Value;

    /// Fetch IMS image ref --> https://apidocs.svc.cscs.ch/paas/ims/operation/get_v3_image/
    /// If filtering by HSM group, then image name must include HSM group name (It assumms each image
    /// is built for a specific cluster based on ansible vars used by the CFS session). The reason
    /// for this is because CSCS staff deletes all CFS sessions every now and then...
    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        hsm_group_name_opt: Option<&String>,
        image_id_opt: Option<&str>,
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

        let api_url = if let Some(image_id) = image_id_opt {
            shasta_base_url.to_owned() + "/ims/v3/images/" + image_id
        } else {
            shasta_base_url.to_owned() + "/ims/v3/images"
        };

        let resp = client
            .get(api_url)
            // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
            .bearer_auth(shasta_token)
            .send()
            .await?;

        let mut json_response: Value = if resp.status().is_success() {
            serde_json::from_str(&resp.text().await?)?
        } else {
            return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        };

        let image_value_vec: &mut Vec<Value> = json_response.as_array_mut().unwrap();

        if let Some(hsm_group_name) = hsm_group_name_opt {
            image_value_vec.retain(|image_value| {
                image_value["name"]
                    .as_str()
                    .unwrap()
                    .contains(hsm_group_name)
            });
        }

        Ok(image_value_vec.to_vec())
    }

    // Delete IMS image using CSM API "soft delete" --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_all_v3_images/
    pub async fn delete(
        shasta_token: &str,
        shasta_base_url: &str,
        image_id: &str,
    ) -> Result<Value, Box<dyn Error>> {
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

        let api_url = shasta_base_url.to_owned() + "/ims/v3/images/" + image_id;

        let resp = client
            .delete(api_url)
            // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
            .bearer_auth(shasta_token)
            .send()
            .await?;

        if resp.status().is_success() {
            let response = &resp.text().await?;
            Ok(serde_json::from_str(response)?)
        } else {
            eprintln!("FAIL request: {:#?}", resp);
            let response: String = resp.text().await?;
            eprintln!("FAIL response: {:#?}", response);
            Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }
}
