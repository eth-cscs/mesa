use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LastAction {
    action: Option<String>,
    #[serde(rename = "numAttempts")]
    num_attempts: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BootArtifacts {
    kernel: Option<String>,
    kernel_parameters: Option<String>,
    rootfs: Option<String>,
    initrd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActualState {
    #[serde(rename = "bootArtifacts")]
    boot_artifacts: Option<BootArtifacts>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DesiredState {
    #[serde(rename = "bootArtifacts")]
    boot_artifacts: Option<BootArtifacts>,
    configuration: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    id: Option<String>,
    #[serde(rename = "actualState")]
    actual_state: Option<ActualState>,
    #[serde(rename = "desiredState")]
    desired_state: Option<DesiredState>,
    #[serde(rename = "lastAction")]
    last_action: Option<LastAction>,
    enabled: Option<String>,
    error: Option<String>,
}

pub mod http_client {

    use std::error::Error;

    use serde_json::Value;

    use super::Component;

    pub async fn get_single_component(
        shasta_token: &str,
        shasta_base_url: &str,
        component_id: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let client;

        let client_builder = reqwest::Client::builder().danger_accept_invalid_certs(true);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy.
            log::debug!("SOCKS5 enabled");
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/components/" + component_id;

        let resp = client
            .get(api_url)
            // .get(format!("{}{}{}", shasta_base_url, "/cfs/v2/components/", component_id))
            .bearer_auth(shasta_token)
            .send()
            .await?
            .text()
            .await?;

        let json_response: Value = serde_json::from_str(&resp)?;

        Ok(json_response)
    }

    pub async fn get_multiple_components(
        shasta_token: &str,
        shasta_base_url: &str,
        components_ids: Option<&str>,
        status: Option<&str>,
        // enabled: Option<bool>,
        /* cfs_configuration_name: Option<&str>,
        cfs_configuration_details: Option<bool>, */
        // tags: Option<&str>,
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

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

        let resp = client
            .get(api_url)
            .query(&[("ids", components_ids), ("status", status)])
            // .get(format!("{}{}{}", shasta_base_url, "/cfs/v2/components/", component_id))
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

    pub async fn post_component(
        shasta_token: &str,
        shasta_base_url: &str,
        component: Component,
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

        let api_url =
            shasta_base_url.to_owned() + "/cfs/v2/component/" + &component.clone().id.unwrap();

        let resp = client
            .post(api_url)
            .bearer_auth(shasta_token)
            .json(&component)
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

    pub async fn post_component_list(
        shasta_token: &str,
        shasta_base_url: &str,
        component_list: Vec<Component>,
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

        let api_url = shasta_base_url.to_owned() + "/cfs/v2/components";

        let resp = client
            .post(api_url)
            .bearer_auth(shasta_token)
            .json(&component_list)
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

pub mod utils {

    use super::{Component, DesiredState};

    pub async fn update_component_desired_configuration(
        shasta_token: &str,
        shasta_base_url: &str,
        xname: &str,
        desired_configuration: &str,
    ) {
        let desired_state = DesiredState {
            boot_artifacts: None,
            configuration: Some(desired_configuration.to_string()),
        };
        let component = Component {
            desired_state: Some(desired_state),
            id: Some(xname.to_string()),
            actual_state: None,
            last_action: None,
            enabled: None,
            error: None,
        };

        crate::shasta::cfs::component::http_client::post_component(
            shasta_token,
            shasta_base_url,
            component,
        )
        .await;
    }

    pub async fn update_component_list_desired_configuration(
        shasta_token: &str,
        shasta_base_url: &str,
        xnames: Vec<String>,
        desired_configuration:&str
    ) {
        let mut component_list = Vec::new();

        for xname in xnames {
            let desired_state = DesiredState {
                boot_artifacts: None,
                configuration: Some(desired_configuration.to_string()),
            };
            let component = Component {
                desired_state: Some(desired_state),
                id: Some(xname),
                actual_state: None,
                last_action: None,
                enabled: None,
                error: None,
            };

            component_list.push(component);
        }

        crate::shasta::cfs::component::http_client::post_component_list(
            shasta_token,
            shasta_base_url,
            component_list,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn update_desired_configuration() {
        let token = "--REDACTED--";

        super::utils::update_component_desired_configuration(
            token,
            "https://api.cmn.alps.cscs.ch/apis",
            "x1001c1s5b1n1",
            "test!",
        )
        .await;
    }
}
