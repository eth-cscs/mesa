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

        use crate::capmc::{self, r#struct::PowerStatus};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason_opt: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Power OFF nodes: {:?}", xname_vec);

            let power_off = PowerStatus::new(reason_opt, xname_vec, force, None);

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
        }

        /// Shut down a node
        /// This is  sync call meaning it won't return untill the target is down
        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason_opt: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            // Check Nodes are shutdown
            let mut node_status_value = capmc::http_client::node_power_status::post(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &xname_vec,
            )
            .await
            .unwrap();

            let mut node_off_vec: Vec<String> = node_status_value["off"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|xname: &Value| xname.as_str().unwrap().to_string())
                .collect();

            // Check all nodes are OFF
            let mut i = 0;
            let max = 60;
            let delay_secs = 3;
            while i <= max && xname_vec.iter().any(|xname| !node_off_vec.contains(xname)) {
                let _ = post(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    xname_vec.clone(),
                    reason_opt.clone(),
                    force,
                )
                .await;

                tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

                node_status_value = capmc::http_client::node_power_status::post(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &xname_vec,
                )
                .await
                .unwrap();

                node_off_vec = node_status_value["off"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|xname: &Value| xname.as_str().unwrap().to_string())
                    .collect();

                println!(
                    "Node(s) in power state OFF: {:?}. Waiting nodes to shutdown. Trying again in {} seconds. Attempt {} of {}",
                    node_off_vec,
                    delay_secs,
                    i + 1,
                    max
                );

                i += 1;
            }

            println!("Node(s) power state OFF: {:?}", node_off_vec);

            Ok(node_status_value)
        }
    }

    pub mod node_power_on {
        use core::time;

        use serde_json::Value;

        use crate::capmc::{self, r#struct::PowerStatus};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason: Option<String>,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Power ON nodes: {:?}", xname_vec);

            let power_on = PowerStatus::new(reason, xname_vec, false, None);

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
        }

        /// Power ON a group of nodes
        /// This is  sync call meaning it won't return untill all nodes are ON
        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason: Option<String>,
        ) -> Result<Value, reqwest::Error> {
            // Check Nodes are shutdown
            let mut node_status_value = capmc::http_client::node_power_status::post(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &xname_vec,
            )
            .await
            .unwrap();

            let mut node_on_vec: Vec<String> = node_status_value["on"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|xname: &Value| xname.as_str().unwrap().to_string())
                .collect();

            // Check all nodes are OFF
            let mut i = 0;
            let max = 60;
            let delay_secs = 3;
            while i <= max && xname_vec.iter().any(|xname| !node_on_vec.contains(xname)) {
                let _ = post(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    xname_vec.clone(),
                    reason.clone(),
                )
                .await;

                tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

                node_status_value = capmc::http_client::node_power_status::post(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &xname_vec,
                )
                .await
                .unwrap();

                node_on_vec = node_status_value["on"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|xname: &Value| xname.as_str().unwrap().to_string())
                    .collect();

                println!(
                    "Node(s) in power state ON: {:?}. Waiting nodes to power on. Trying again in {} seconds. Attempt {} of {}",
                    node_on_vec,
                    delay_secs,
                    i + 1,
                    max
                );

                i += 1;
            }

            println!("Node(s) power state ON: {:?}", node_on_vec);

            Ok(node_status_value)
        }
    }

    pub mod node_power_reset {

        use serde_json::Value;

        use crate::capmc::{self, r#struct::PowerStatus};

        pub async fn post(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            let node_restart = PowerStatus::new(reason, xname_vec, force, None);

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
        }

        pub async fn post_sync(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<String>,
            reason_opt: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            log::info!("Power RESET node: {:?}", xname_vec);

            let _ = capmc::http_client::node_power_off::post_sync(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                xname_vec.clone(),
                reason_opt.clone(),
                force,
            )
            .await;

            capmc::http_client::node_power_on::post_sync(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                xname_vec,
                reason_opt,
            )
            .await
        }

        /// Power RESET a group of nodes
        /// This is  sync call meaning it won't return untill all nodes are ON
        pub async fn post_sync_vec(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: Vec<String>,
            reason_opt: Option<String>,
            force: bool,
        ) -> Result<Value, reqwest::Error> {
            let mut nodes_reseted = Vec::new();

            let mut tasks = tokio::task::JoinSet::new();

            for xname in xnames {
                let shasta_token_string = shasta_token.to_string();
                let shasta_base_url_string = shasta_base_url.to_string();
                let shasta_root_cert_vec = shasta_root_cert.to_vec();
                let reason_cloned = reason_opt.clone();

                tasks.spawn(async move {
                    post_sync(
                        &shasta_token_string,
                        &shasta_base_url_string,
                        &shasta_root_cert_vec,
                        vec![xname],
                        reason_cloned,
                        force,
                    )
                    .await
                    .unwrap()
                });
            }

            while let Some(message) = tasks.join_next().await {
                if let Ok(node_power_status) = message {
                    nodes_reseted.push(node_power_status);
                }
            }

            Ok(serde_json::to_value(nodes_reseted).unwrap())
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
        }
    }
}
