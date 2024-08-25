pub mod transitions {

    pub mod r#struct {

        use serde::{Deserialize, Serialize};

        use crate::error::Error;

        #[derive(Debug, Serialize, Deserialize, Default)]
        pub struct Location {
            pub xname: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "deputyKey")]
            pub deputy_key: Option<String>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub enum Operation {
            #[serde(rename = "on")]
            On,
            #[serde(rename = "off")]
            Off,
            #[serde(rename = "soft-off")]
            SoftOff,
            #[serde(rename = "soft-restart")]
            SoftRestart,
            #[serde(rename = "hard-restart")]
            HardRestart,
            #[serde(rename = "init")]
            Init,
            #[serde(rename = "force-off")]
            ForceOff,
        }

        impl Operation {
            /* pub fn to_string(&self) -> String {
                match self {
                    Operation::On => "on".to_string(),
                    Operation::Off => "off".to_string(),
                    Operation::SoftOff => "soft-off".to_string(),
                    Operation::SoftRestart => "soft-restart".to_string(),
                    Operation::HardRestart => "hard-restart".to_string(),
                    Operation::Init => "init".to_string(),
                    Operation::ForceOff => "force-off".to_string(),
                }
            } */

            pub fn from_str(operation: &str) -> Result<Operation, Error> {
                match operation {
                    "on" => Ok(Operation::On),
                    "off" => Ok(Operation::Off),
                    "soft-off" => Ok(Operation::SoftOff),
                    "soft-restart" => Ok(Operation::SoftRestart),
                    "hard-restart" => Ok(Operation::HardRestart),
                    "init" => Ok(Operation::Init),
                    "force-off" => Ok(Operation::ForceOff),
                    _ => Err(Error::Message("Operation not valid".to_string())),
                }
            }
        }

        /* impl FromStr for Operation {
            type Err = Error;

            fn from_str(operation: &str) -> Result<Operation, Error> {
                Self::from_str(operation)
            }
        } */

        #[derive(Debug, Serialize, Deserialize)]
        pub struct Transition {
            pub operation: Operation,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskDeadlineMinutes")]
            pub task_deadline_minutes: Option<usize>,
            pub location: Vec<Location>,
        }
    }

    pub mod http_client {
        use std::time;

        use serde_json::Value;

        use crate::{
            error::Error,
            pcs::transitions::r#struct::{Location, Operation},
        };

        use super::r#struct::Transition;
        pub async fn get(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
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

            let api_url = format!("{}/power-control/v1/transitions", shasta_base_url);

            let response = client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                let resp_payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                serde_json::from_value::<Vec<Value>>(resp_payload["transitions"].clone())
                    .map_err(|error| Error::SerdeError(error))
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn get_by_id(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            id: &str,
        ) -> Result<Value, Error> {
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

            let api_url = format!("{}/power-control/v1/transitions/{}", shasta_base_url, id);

            let response = client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                let payload = response
                    .json()
                    .await
                    .map_err(|error| Error::NetError(error));

                log::debug!("PCS transition details\n{:#?}", payload);

                payload
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn post(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            operation: &str,
            xname_vec: &Vec<String>,
        ) -> Result<Value, Error> {
            log::info!("Create PCS transition '{}' on {:?}", operation, xname_vec);

            //Create request payload
            //
            // Create 'location' list with all the xnames to operate
            let mut location_vec: Vec<Location> = Vec::new();

            for xname in xname_vec {
                let location: Location = Location {
                    xname: xname.to_string(),
                    deputy_key: None,
                };

                location_vec.push(location);
            }

            // Create 'transition'
            let request_payload = Transition {
                operation: Operation::from_str(operation)?,
                task_deadline_minutes: None,
                location: location_vec,
            };

            // Build http client
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url = shasta_base_url.to_owned() + "/power-control/v1/transitions";

            // Submit call to http api
            let response = client
                .post(api_url)
                .json(&request_payload)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(response.json::<Value>().await.unwrap())
            } else {
                let payload = response.json().await.map_err(|e| Error::NetError(e))?;

                Err(Error::CsmError(payload))
            }
        }

        // Creates a task on CSM for power management nodes.
        // Returns a serde_json::Value with the power task management
        pub async fn post_block(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            operation: &str,
            xname_vec: &Vec<String>,
        ) -> Result<Value, Error> {
            let node_reset = post(
                shasta_base_url,
                shasta_token,
                shasta_root_cert,
                operation,
                xname_vec,
            )
            .await?;

            let transition_id = node_reset["transitionID"].as_str().unwrap();

            log::info!("PCS transition ID: {}", transition_id);

            let power_management_status: Value = wait_to_complete(
                shasta_base_url,
                shasta_token,
                shasta_root_cert,
                xname_vec,
                transition_id,
            )
            .await?;

            Ok(power_management_status)
        }

        pub async fn wait_to_complete(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            xname_vec: &Vec<String>,
            transition_id: &str,
        ) -> Result<Value, Error> {
            let mut transition_status = "";

            let mut transition: serde_json::Value = get_by_id(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                transition_id,
            )
            .await?;

            let mut i = 1;
            let max_attempt = 300;

            while i <= max_attempt && transition_status != "completed" {
                // Check PCS transition status
                transition = get_by_id(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    transition_id,
                )
                .await?;

                transition_status = transition["transitionStatus"].as_str().unwrap();

                let operation = transition["operation"].as_str().unwrap();

                let failed = transition
                    .pointer("/taskCounts/failed")
                    .unwrap()
                    .as_number()
                    .unwrap();

                let in_progress = transition
                    .pointer("/taskCounts/in-progress")
                    .unwrap()
                    .as_number()
                    .unwrap();

                /* let new = transition
                .pointer("/taskCounts/new")
                .unwrap()
                .as_number()
                .unwrap(); */

                let succeeded = transition
                    .pointer("/taskCounts/succeeded")
                    .unwrap()
                    .as_number()
                    .unwrap();

                let total = transition
                    .pointer("/taskCounts/total")
                    .unwrap()
                    .as_number()
                    .unwrap();

                /* let un_supported = transition
                .pointer("/taskCounts/un-supported")
                .unwrap()
                .as_number()
                .unwrap(); */

                eprintln!(
                    "Power '{}' summary - status: {}, failed: {}, in-progress: {}, succeeded: {}, total: {}. Attempt {} of {}",
                    operation, transition_status, failed, in_progress, succeeded, total, i, max_attempt
                );

                tokio::time::sleep(time::Duration::from_secs(3)).await;
                i += 1;
            }

            if transition_status == "completed" {
                Ok(transition)
            } else {
                Err(Error::CsmError(transition))
            }
        }
    }
}

pub mod power_status {

    pub mod r#struct {
        use serde::{Deserialize, Serialize};

        use crate::pcs::transitions::r#struct::Operation;

        #[derive(Debug, Serialize, Deserialize)]
        pub enum PowerState {
            On,
            Off,
            Undefined,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub enum ManagementState {
            Unavailable,
            Available,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct PowerStatus {
            xname: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            power_state_filter: Option<PowerState>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "powerState")]
            power_state: Option<PowerState>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "management_state")]
            management_state: Option<ManagementState>,
            #[serde(skip_serializing_if = "Option::is_none")]
            management_state_filter: Option<ManagementState>,
            #[serde(skip_serializing_if = "Option::is_none")]
            error: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "supportedPowerTransitions")]
            supported_power_transitions: Option<Operation>,
            last_updated: String,
        }
    }

    pub mod http_client {
        use serde_json::Value;

        use crate::error::Error;

        use super::r#struct::PowerStatus;

        pub async fn get(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            xname_vec_opt: Option<&[&str]>,
            power_state_filter_opt: Option<&str>,
            management_state_filter_opt: Option<&str>,
        ) -> Result<PowerStatus, Error> {
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

            let api_url = format!("{}/power-control/v1/power-status", shasta_base_url);

            let xname_vec_str_opt: Option<String> =
                xname_vec_opt.map(|xname_vec| xname_vec.join(","));

            let response = client
                .get(api_url)
                .query(&[
                    ("xname", xname_vec_str_opt),
                    (
                        "powerStateFilter",
                        power_state_filter_opt.map(|value| value.to_string()),
                    ),
                    (
                        "managementStateFilter",
                        management_state_filter_opt.map(|value| value.to_string()),
                    ),
                ])
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                response
                    .json()
                    .await
                    .map_err(|error| Error::NetError(error))
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn post(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            power_status: PowerStatus,
        ) -> Result<PowerStatus, Error> {
            log::info!("Create PCS power status:\n'{:#?}'", power_status);
            log::debug!("Create PCS power status:\n{:#?}", power_status);

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

            let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-status";

            let response = client
                .put(api_url)
                .json(&power_status)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|e| Error::NetError(e))?;

            if response.status().is_success() {
                Ok(response.json().await.map_err(|e| Error::NetError(e))?)
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|e| Error::NetError(e))?;

                Err(Error::CsmError(payload))
            }
        }
    }
}

pub mod power_cap {

    pub mod r#struct {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        pub struct PowerCapTaskList {
            pub tasks: Vec<PowerCapTaskInfo>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct TaskCounts {
            pub total: usize,
            pub new: usize,
            pub in_progress: usize,
            pub failed: usize,
            pub succeeded: usize,
            pub un_supported: usize,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct Limit {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "hostsLimitMax")]
            pub hosts_limit_max: Option<usize>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "hostsLimitMin")]
            pub hosts_limit_min: Option<usize>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "powerupPower")]
            pub powerup_power: Option<usize>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct PowerCapLimit {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "currentValue")]
            pub current_value: Option<usize>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "mamximumValue")]
            pub maximum_value: Option<usize>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "minimumValue")]
            pub mnimum_value: Option<usize>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct PowerCapComponent {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub xname: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub error: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub limits: Option<Limit>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "power_cap_limits")]
            pub power_cap_limits: Option<PowerCapLimit>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct PowerCapTaskInfo {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskId")]
            pub task_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub r#type: Option<String>, // TODO: convert to enum. Valid values are `snapshot` and `patch`
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskCreateTime")]
            pub task_create_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "automaticExpirationTime")]
            pub automatic_expiration_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskStatus")]
            pub task_status: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskCounts")]
            pub task_counts: Option<TaskCounts>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub components: Option<Vec<PowerCapComponent>>,
        }
    }

    pub mod http_client {
        use serde_json::Value;

        use crate::error::Error;

        use super::r#struct::{PowerCapComponent, PowerCapTaskInfo};

        pub async fn get(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
        ) -> Result<PowerCapTaskInfo, Error> {
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

            let api_url = format!("{}/power-control/v1/power-cap", shasta_base_url);

            let response = client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                response
                    .json()
                    .await
                    .map_err(|error| Error::NetError(error))
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn get_task_id(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            task_id: &str,
        ) -> Result<PowerCapTaskInfo, Error> {
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

            let api_url = format!("{}/power-control/v1/power-cap/{}", shasta_base_url, task_id);

            let response = client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                response
                    .json()
                    .await
                    .map_err(|error| Error::NetError(error))
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn post_snapshot(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            xname_vec: Vec<&str>,
        ) -> Result<PowerCapTaskInfo, Error> {
            log::info!("Create PCS power snapshot for nodes:\n{:?}", xname_vec);
            log::debug!("Create PCS power snapshot for nodes:\n{:?}", xname_vec);

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

            let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-cap/snapshot";

            let response = client
                .put(api_url)
                .json(&serde_json::json!({
                    "xnames": xname_vec
                }))
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|e| Error::NetError(e))?;

            if response.status().is_success() {
                Ok(response.json().await.map_err(|e| Error::NetError(e))?)
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|e| Error::NetError(e))?;

                Err(Error::CsmError(payload))
            }
        }

        pub async fn patch(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            power_cap: Vec<PowerCapComponent>,
        ) -> Result<PowerCapTaskInfo, Error> {
            log::info!("Create PCS power cap:\n{:#?}", power_cap);
            log::debug!("Create PCS power cap:\n{:#?}", power_cap);

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

            let api_url = shasta_base_url.to_owned() + "/power-control/v1/power-cap/snapshot";

            let response = client
                .put(api_url)
                .json(&power_cap)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|e| Error::NetError(e))?;

            if response.status().is_success() {
                Ok(response.json().await.map_err(|e| Error::NetError(e))?)
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|e| Error::NetError(e))?;

                Err(Error::CsmError(payload))
            }
        }
    }
}
