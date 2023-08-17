use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cloneUrl")]
    clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sesisonName")]
    session_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Vec<State>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stateAppend")]
    state_append: Option<State>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "desiredConfig")]
    desired_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "errorCount")]
    error_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "retryPolicy")]
    retry_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    // tags: TODO: this is supposed to be an object??? https://csm12-apidocs.svc.cscs.ch/paas/cfs/operation/patch_component/#!path=tags&t=request
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

    pub async fn patch_component(
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
            shasta_base_url.to_owned() + "/cfs/v2/components/" + &component.clone().id.unwrap();

        let resp = client
            .patch(api_url)
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

    pub async fn patch_component_list(
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
            .patch(api_url)
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

    use super::Component;

    pub async fn update_component_desired_configuration(
        shasta_token: &str,
        shasta_base_url: &str,
        xname: &str,
        desired_configuration: &str,
        enabled: bool
    ) {
        let component = Component {
            id: Some(xname.to_string()),
            desired_config: Some(desired_configuration.to_string()),
            state: None,
            state_append: None,
            error_count: None,
            retry_policy: None,
            enabled: Some(enabled),
        };

        let _ = crate::shasta::cfs::component::http_client::patch_component(
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
        desired_configuration: &str,
        enabled: bool
    ) {
        let mut component_list = Vec::new();

        for xname in xnames {
            let component = Component {
                id: Some(xname.to_string()),
                desired_config: Some(desired_configuration.to_string()),
                state: None,
                state_append: None,
                error_count: None,
                retry_policy: None,
                enabled: Some(enabled),
            };

            component_list.push(component);
        }

        let _ = crate::shasta::cfs::component::http_client::patch_component_list(
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
            true
        )
        .await;
    }
}
