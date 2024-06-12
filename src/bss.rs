#[cfg(feature = "ochami")]
pub mod bootparameters {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Serialize, Deserialize, Default, Clone)]
    pub struct BootParameters {
        #[serde(default)]
        pub hosts: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub macs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nids: Option<Vec<u32>>,
        #[serde(default)]
        pub params: String,
        #[serde(default)]
        pub kernel: String,
        #[serde(default)]
        pub initrd: String,
        #[serde(rename = "cloud-init")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cloud_init: Option<Value>,
    }

    /* let boot_parameters = BootParameters::new(
        xnames,
        macs_opt,
        nids_opt,
        params,
        kernel,
        initrd,
        cloud_init_opt,
    ); */

    impl BootParameters {
        pub fn new(
            hosts: Vec<&str>,
            macs: Option<Vec<&str>>,
            nids: Option<Vec<&str>>,
            params: &str,
            kernel: &str,
            initrd: &str,
            cloud_init_opt: Option<&str>,
        ) -> Self {
            BootParameters {
                hosts: hosts.iter().map(|value| value.to_string()).collect(),
                macs: macs.map(|mac_vec| mac_vec.iter().map(|value| value.to_string()).collect()),
                nids: nids.map(|nid_vec| {
                    nid_vec
                        .iter()
                        .map(|value| value.parse::<u32>().unwrap())
                        .collect()
                }),
                params: params.to_string(),
                kernel: kernel.to_string(),
                initrd: initrd.to_string(),
                cloud_init: cloud_init_opt
                    .map(|cloud_init| serde_json::to_value(cloud_init).unwrap()),
            }
        }

        /// Returns the image id. This function may fail since it assumes kernel path has the following
        /// format `s3://xxxxx/<image id>/kernel`
        pub fn get_boot_image(&self) -> String {
            let mut path_elem_vec = self.kernel.split("/").skip(3);

            let mut image_id: String = path_elem_vec.next().unwrap_or_default().to_string();

            for path_elem in path_elem_vec {
                if !path_elem.eq("kernel") {
                    image_id = format!("{}/{}", image_id, path_elem);
                } else {
                    break;
                }
            }

            image_id
        }

        pub fn set_boot_image(&mut self, new_image_id: &str) {
            self.params = self
                .params
                .split_whitespace()
                .map(|kernel_param| {
                    if kernel_param.contains("metal.server=s3://boot-images/") {
                        // NCN node
                        let aux = kernel_param
                            .trim_start_matches("metal.server=s3://boot-images/")
                            .split_once('/')
                            .unwrap()
                            .1;

                        format!("metal.server=s3://boot-images/{}/{}", new_image_id, aux)
                    } else if kernel_param.contains("root=craycps-s3:s3://boot-images/") {
                        // CN node
                        let aux = kernel_param
                            .trim_start_matches("root=craycps-s3:s3://boot-images/")
                            .split_once('/')
                            .unwrap_or_default() // NCN has root=live:LABEL=SQFSRAID
                            .1;

                        format!("root=craycps-s3:s3://boot-images/{}/{}", new_image_id, aux)
                    } else if kernel_param.contains("nmd_data=") {
                        // CN node
                        let aux = kernel_param
                            .trim_start_matches("nmd_data=url=s3://boot-images/")
                            .split_once('/')
                            .unwrap()
                            .1;

                        format!("nmd_data=url=s3://boot-images/{}/{}", new_image_id, aux)
                    } else {
                        kernel_param.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            self.kernel = format!("s3://boot-images/{}/kernel", new_image_id);

            self.initrd = format!("s3://boot-images/{}/initrd", new_image_id);
        }
    }

    pub mod http_client {

        use serde_json::Value;
        use tokio::sync::Semaphore;

        use core::result::Result;
        use std::sync::Arc;

        use crate::error::Error;

        use super::BootParameters;

        /// Change nodes boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/put/
        pub async fn put(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            boot_parameters: BootParameters,
        ) -> Result<Vec<Value>, reqwest::Error> {
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

            let api_url = format!("{}/bss/boot/v1/bootparameters", shasta_base_url);

            log::debug!(
                "request payload:\n{}",
                serde_json::to_string_pretty(&boot_parameters).unwrap()
            );

            client
                .put(api_url)
                .json(&boot_parameters)
                .bearer_auth(shasta_token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }

        pub async fn patch(
            shasta_base_url: &str,
            shasta_token: &str,
            shasta_root_cert: &[u8],
            // xnames: &[String],
            // params: Option<&String>,
            // kernel: Option<&String>,
            // initrd: Option<&String>,
            boot_parameters: &BootParameters,
        ) -> Result<Vec<Value>, reqwest::Error> {
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

            let api_url = format!("{}/bss/boot/v1/bootparameters", shasta_base_url);

            client
                .patch(api_url)
                .json(&boot_parameters)
                // .json(&serde_json::json!({"hosts": xnames, "params": params, "kernel": kernel, "initrd": initrd})) // Encapsulating configuration.layers
                .bearer_auth(shasta_token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }

        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: &[String],
        ) -> Result<Vec<BootParameters>, reqwest::Error> {
            let chunk_size = 30;

            let mut boot_params_vec = Vec::new();

            let mut tasks = tokio::task::JoinSet::new();

            let sem = Arc::new(Semaphore::new(10)); // CSM 1.3.1 higher number of concurrent tasks won't

            for sub_node_list in xnames.chunks(chunk_size) {
                let shasta_token_string = shasta_token.to_string();
                let shasta_base_url_string = shasta_base_url.to_string();
                let shasta_root_cert_vec = shasta_root_cert.to_vec();

                // let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

                let permit = Arc::clone(&sem).acquire_owned().await;

                let node_vec = sub_node_list.to_vec();

                tasks.spawn(async move {
                    let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

                    get_boot_params(
                        &shasta_token_string,
                        &shasta_base_url_string,
                        &shasta_root_cert_vec,
                        // &hsm_subgroup_nodes_string,
                        &node_vec,
                    )
                    .await
                    .unwrap()
                });
            }

            while let Some(message) = tasks.join_next().await {
                if let Ok(mut node_status_vec) = message {
                    boot_params_vec.append(&mut node_status_vec);
                }
            }

            Ok(boot_params_vec)
        }

        /// Get node boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/get/
        pub async fn get_boot_params(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: &[String],
        ) -> Result<Vec<BootParameters>, Error> {
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

            let url_api = format!("{}/bss/boot/v1/bootparameters", shasta_base_url.to_string());

            let params: Vec<_> = xnames.iter().map(|xname| ("name", xname)).collect();

            let response = client
                .get(url_api)
                .query(&params)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                response
                    .json::<Vec<BootParameters>>()
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

    pub mod utils {
        use serde_json::Value;

        use super::BootParameters;

        pub fn find_boot_params_related_to_node(
            node_boot_params_list: &[BootParameters],
            node: &String,
        ) -> Option<BootParameters> {
            node_boot_params_list
                .iter()
                .find(|node_boot_param| node_boot_param.hosts.iter().any(|host| host.eq(node)))
                .cloned()
        }

        /// Get Image ID from kernel field
        #[deprecated(
            since = "1.26.6",
            note = "Please convert from serde_json::Value to struct BootParameters use function `BootParameters::get_boot_image` instead"
        )]
        pub fn get_image_id(node_boot_params: &Value) -> String {
            serde_json::from_value::<BootParameters>(node_boot_params.clone())
                .unwrap()
                .get_boot_image()
        }
    }
}
