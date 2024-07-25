pub mod transitions {

    pub mod r#struct {
        use std::str::FromStr;

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
            On,
            Off,
            SoftOff,
            SoftRestart,
            HardRestart,
            Init,
            ForceOff,
        }

        impl Operation {
            pub fn to_string(&self) -> String {
                match self {
                    Operation::On => "on".to_string(),
                    Operation::Off => "off".to_string(),
                    Operation::SoftOff => "soft-off".to_string(),
                    Operation::SoftRestart => "soft-restart".to_string(),
                    Operation::HardRestart => "hard-restart".to_string(),
                    Operation::Init => "init".to_string(),
                    Operation::ForceOff => "force-off".to_string(),
                }
            }

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

        impl FromStr for Operation {
            type Err = Error;

            fn from_str(operation: &str) -> Result<Operation, Error> {
                Self::from_str(operation)
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct Transition {
            pub operation: Operation,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "taskDeadlineMinutes")]
            pub task_deadline_minutes: Option<usize>,
            pub location: Location,
        }
    }

    pub mod http_client {
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
        ) -> Result<Vec<Transition>, Error> {
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

                serde_json::from_value::<Vec<Transition>>(resp_payload["transitions"].clone())
                    .map_err(|error| Error::Message(error.to_string())) // TODO: Fix
                                                                        // this by adding a fiend in Error compatible with serde_json::Error
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
        ) -> Result<Transition, Error> {
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
            operation: &str,
            xname: &str,
        ) -> Result<Transition, Error> {
            log::info!("Create PCS transition '{}'", operation);
            log::debug!("Create PCS transition request:\n{:#?}", operation);

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

            let api_url = shasta_base_url.to_owned() + "/power-control/v1/transitions";

            let request_payload = Transition {
                operation: Operation::from_str(operation)?,
                task_deadline_minutes: None,
                location: Location {
                    xname: xname.to_string(),
                    deputy_key: None,
                },
            };

            let response = client
                .put(api_url)
                .json(&request_payload)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(response
                    .json()
                    .await
                    .map_err(|error| Error::NetError(error))?)
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::CsmError(payload))
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
    }
}
