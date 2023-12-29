pub mod r#struct {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default)]
    pub struct PowerStatus {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        xnames: Vec<String>,
        force: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        recursive: Option<bool>,
    }

    impl PowerStatus {
        pub fn new(
            reason: Option<String>,
            xnames: Vec<String>,
            force: bool,
            recursive: Option<bool>,
        ) -> Self {
            Self {
                reason,
                xnames,
                force,
                recursive,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Default)]
    pub struct NodeStatus {
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        xnames: Option<Vec<String>>,
    }

    impl NodeStatus {
        pub fn new(
            filter: Option<String>,
            xnames: Option<Vec<String>>,
            source: Option<String>,
        ) -> Self {
            Self {
                filter,
                source,
                xnames,
            }
        }
    }
}

pub mod http_client {

    pub mod node_power_off {

        use core::time;

        use serde_json::Value;

        use crate::{capmc::r#struct::PowerStatus, hsm};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Shutting down nodes: {:?}", xnames);

            let power_off = PowerStatus::new(reason, xnames, force, None);

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

            let api_url = shasta_base_url.to_owned() + "/capmc/capmc/v1/xname_off";

            let resp = client
                .post(api_url)
                .bearer_auth(shasta_token)
                .json(&power_off)
                .send()
                .await?;

            match resp.error_for_status() {
                Ok(response) => Ok(response.json::<Value>().await?),
                Err(error) => Err(error),
            }

            /* if resp.status().is_success() {
                Ok(resp.json::<Value>().await?)
            } else {
                Err(resp.json::<Value>().await?["detail"]
                    .as_str()
                    .unwrap()
                    .into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
            } */
        }

        /// Shut down a node
        /// This is  sync call meaning it won't return untill the target is down
        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            let xname_list: Vec<String> = xnames.into_iter().collect();
            // Create CAPMC operation shutdown
            let capmc_power_off_nodes_resp = post(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                xname_list.clone(),
                reason,
                force,
            )
            .await;

            log::debug!("Shutdown nodes resp:\n{:#?}", capmc_power_off_nodes_resp);

            // Check Nodes are shutdown
            let mut nodes_status_resp =
                hsm::component_status::shasta::http_client::get_components_status(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &xname_list,
                )
                .await;

            log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

            // Check all nodes are OFF
            let mut i = 0;
            let max = 60;
            let delay_secs = 3;
            while i <= max
                && !nodes_status_resp.as_ref().unwrap()["Components"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|node| node["State"].as_str().unwrap().to_string().eq("Off"))
            {
                print!(
                    "\rWaiting nodes to shutdown. Trying again in {} seconds. Attempt {} of {}",
                    delay_secs,
                    i + 1,
                    max
                );

                tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

                i += 1;

                log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

                nodes_status_resp =
                    hsm::component_status::shasta::http_client::get_components_status(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        &xname_list,
                    )
                    .await;
            }

            println!();

            log::debug!("node status resp:\n{:#?}", nodes_status_resp);

            capmc_power_off_nodes_resp
        }
    }

    pub mod node_power_on {
        use core::time;

        use serde_json::Value;

        use crate::{capmc::r#struct::PowerStatus, hsm};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Powering on nodes: {:?}", xnames);

            let power_on = PowerStatus::new(reason, xnames, force, None);

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url = shasta_base_url.to_owned() + "/capmc/capmc/v1/xname_on";

            let resp = client
                .post(api_url)
                .bearer_auth(shasta_token)
                .json(&power_on)
                .send()
                .await?;

            match resp.error_for_status() {
                Ok(response) => Ok(response.json::<Value>().await?),
                Err(error) => Err(error),
            }
            /* if resp.status().is_success() {
                Ok(resp.json::<Value>().await?)
            } else {
                resp.error_for_status()
            } */
        }

        /// Power ON a group of nodes
        /// This is  sync call meaning it won't return untill all nodes are ON
        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            let xname_list: Vec<String> = xnames.into_iter().collect();
            // Create CAPMC operation shutdown
            let capmc_power_on_nodes_resp = post(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                xname_list.clone(),
                reason,
                force,
            )
            .await;

            log::debug!("Power ON nodes resp:\n{:#?}", capmc_power_on_nodes_resp);

            // Check Nodes are ON
            let mut nodes_status_resp =
                hsm::component_status::shasta::http_client::get_components_status(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &xname_list,
                )
                .await;

            log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

            // Check all nodes are ON
            let mut i = 0;
            let max = 60;
            let delay_secs = 3;
            while i <= max
                && !nodes_status_resp.as_ref().unwrap()["Components"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|node| node["State"].as_str().unwrap().to_string().eq("On"))
            {
                print!(
                    "\rWaiting nodes to power on. Trying again in {} seconds. Attempt {} of {}",
                    delay_secs,
                    i + 1,
                    max
                );

                tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

                i += 1;

                log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

                nodes_status_resp =
                    hsm::component_status::shasta::http_client::get_components_status(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        &xname_list,
                    )
                    .await;
            }

            println!();

            log::debug!("node status resp:\n{:#?}", nodes_status_resp);

            capmc_power_on_nodes_resp
        }
    }

    pub mod node_power_restart {

        use core::time;

        use serde_json::Value;

        use crate::{capmc::r#struct::PowerStatus, hsm};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            reason: Option<&String>,
            xnames: Vec<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Restarting nodes: {:?}", xnames);

            let node_restart = PowerStatus::new(reason.cloned(), xnames, force, None);

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

            let api_url = shasta_base_url.to_owned() + "/capmc/capmc/v1/xname_reinit";

            let resp = client
                .post(api_url)
                .bearer_auth(shasta_token)
                .json(&node_restart)
                .send()
                .await?;

            match resp.error_for_status() {
                Ok(response) => Ok(response.json::<Value>().await?),
                Err(error) => Err(error),
            }

            /* if resp.status().is_success() {
                Ok(resp.json::<Value>().await?)
            } else {
                Err(resp.json::<Value>().await?["detail"]
                    .as_str()
                    .unwrap()
                    .into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
            } */
        }

        /// Power RESET a group of nodes
        /// This is  sync call meaning it won't return untill all nodes are ON
        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason: Option<&String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            let xname_list: Vec<String> = xnames.into_iter().collect();
            // Create CAPMC operation shutdown
            let capmc_power_reset_nodes_resp = post(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                reason,
                xname_list.clone(),
                force,
            )
            .await;

            log::debug!(
                "Power RESET nodes resp:\n{:#?}",
                capmc_power_reset_nodes_resp
            );

            // Check Nodes are ON
            let mut nodes_status_resp =
                hsm::component_status::shasta::http_client::get_components_status(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &xname_list,
                )
                .await;

            log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

            // Check all nodes are ON
            let mut i = 0;
            let max = 60;
            let delay_secs = 3;
            while i <= max
                && !nodes_status_resp.as_ref().unwrap()["Components"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|node| node["State"].as_str().unwrap().to_string().eq("On"))
            {
                print!(
                    "\rWaiting nodes to power on. Trying again in {} seconds. Attempt {} of {}",
                    delay_secs,
                    i + 1,
                    max
                );

                tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

                i += 1;

                log::debug!("nodes_status:\n{:#?}", nodes_status_resp);

                nodes_status_resp =
                    hsm::component_status::shasta::http_client::get_components_status(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        &xname_list,
                    )
                    .await;
            }

            println!();

            log::debug!("node status resp:\n{:#?}", nodes_status_resp);

            capmc_power_reset_nodes_resp
        }
    }

    pub mod node_power_status {

        use serde_json::Value;

        use crate::capmc::r#struct::NodeStatus;

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: &Vec<String>,
        ) -> core::result::Result<Value, reqwest::Error> {
            println!("DEBUG - CHECK NODE POWER STATUS");
            log::info!("Checking nodes status: {:?}", xnames);

            let node_status_payload =
                NodeStatus::new(None, Some(xnames.clone()), Some("hsm".to_string()));

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

            let url_api = shasta_base_url.to_owned() + "/capmc/capmc/v1/get_xname_status";

            let resp = client
                .post(url_api)
                .bearer_auth(shasta_token)
                .json(&node_status_payload)
                .send()
                .await?;

            match resp.error_for_status() {
                Ok(response) => Ok(response.json::<Value>().await?),
                Err(error) => Err(error),
            }

            /* if resp.status().is_success() {
                let resp_json = &resp.json::<Value>().await?;
                Ok(resp_json.clone())
            } else {
                Err(resp.json::<Value>().await?["detail"]
                    .as_str()
                    .unwrap()
                    .into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
            } */
        }
    }
}
