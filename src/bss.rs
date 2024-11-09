#[cfg(feature = "ochami")]
pub mod bootparameters {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::error::Error;

    #[derive(Debug, Serialize, Deserialize, Default, Clone)]
    pub struct BootParameters {
        #[serde(default)]
        pub hosts: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub macs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nids: Option<Vec<u32>>,
        #[serde(default)]
        pub params: String, // FIXME: change type to HashMap<String, String> by using function
        // bss::utils::convert_kernel_params_to_map AND create new method
        // bss::BootParameters::num_kernel_params which returns the list of kernel parameters
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

        /// Update boot image in kernel boot parameters and also in kernel and initrd fields if
        /// exists. Otherwise nothing is changed. This method updates both kernel params related to
        /// NCN and also CN
        /// Returns a boolean that indicates if kernel parameters have change:
        ///  - kernel parameter value changed
        ///  - number of kernel parameters have changed
        pub fn update_boot_image(&mut self, new_image_id: &str) -> Result<bool, Error> {
            let mut changed = false;
            // replace image id in 'root' kernel param

            // convert kernel params to a hashmap
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            // NOTE: CN nodes have UIID image id in 'root' kernel parameter
            // Get `root` kernel parameter and split it by '/'
            let root_kernel_param_rslt = params.get("root");

            let mut root_kernel_param: Vec<&str> = match root_kernel_param_rslt {
                Some(root_kernel_param) => root_kernel_param.split("/").collect::<Vec<&str>>(),
                None => {
                    return Err(Error::Message(
                        "ERROR - The 'root' kernel param is missing from user input".to_string(),
                    ));
                }
            };

            // Replace image id in root kernel param with new image id
            for current_image_id in &mut root_kernel_param {
                // Look for any substring between '/' that matches an UUID formant and take it as
                // the image id
                if uuid::Uuid::try_parse(current_image_id).is_ok() {
                    if *current_image_id != new_image_id {
                        changed = true;
                    }
                    // Replace image id in `root` kernel parameter with new value
                    *current_image_id = new_image_id;
                }
            }

            // Create new `root` kernel param string
            let new_root_kernel_param = root_kernel_param.join("/");

            // Create new kernel parameters
            params
                .entry("root")
                .and_modify(|root_param| *root_param = &new_root_kernel_param);

            self.update_kernel_param("root", &new_root_kernel_param);

            // replace image id in 'nmd_data' kernel param

            // convert kernel params to a hashmap
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            // NOTE: NCN nodes have UUID image id in 'metal.server' kernel parameter
            let mut metal_server_kernel_param: Vec<&str>;
            if let Some(metal_server_data) = params.get("metal.server") {
                metal_server_kernel_param = metal_server_data.split("/").collect();

                for substring in &mut metal_server_kernel_param {
                    if uuid::Uuid::try_parse(substring).is_ok() {
                        *substring = new_image_id;
                        changed = true;
                    }
                }

                let new_metal_server_kernel_param = metal_server_kernel_param.join("/");

                params
                    .entry("metal.server")
                    .and_modify(|metal_server_param| {
                        *metal_server_param = &new_metal_server_kernel_param
                    });

                self.update_kernel_param("metal.server", &new_metal_server_kernel_param);

                // convert kernel params to a hashmap
                params = self
                    .params
                    .split_whitespace()
                    .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                    .collect();
            } else {
            };

            // NOTE: NCN nodes have UUID image id 'nmd_data' kernel parameter
            let mut nmd_kernel_param: Vec<&str>;
            if let Some(nmd_data) = params.get("nmd_data") {
                nmd_kernel_param = nmd_data.split("/").collect();

                for substring in &mut nmd_kernel_param {
                    if uuid::Uuid::try_parse(substring).is_ok() {
                        *substring = new_image_id;
                        changed = true;
                    }
                }

                let new_nmd_kernel_param = nmd_kernel_param.join("/");

                params
                    .entry("nmd_data")
                    .and_modify(|nmd_param| *nmd_param = &new_nmd_kernel_param);

                self.update_kernel_param("nmd_data", &new_nmd_kernel_param);
            } else {
            };

            /* self.params = self
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
                    format!("root=craycps-s3:s3://boot-images/{}/rootfs:etag:dvs:api-gw-service-nmn.local:300:hsn0,nmn0:0", new_image_id)
                    /* let aux = kernel_param
                        .trim_start_matches("root=craycps-s3:s3://boot-images/")
                        .split_once('/')
                        .unwrap_or_default() // NCN has root=live:LABEL=SQFSRAID
                        .1;

                    format!("root=craycps-s3:s3://boot-images/{}/{}", new_image_id, aux) */
                } else if kernel_param.contains("nmd_data=") {
                    // CN node
                    format!("nmd_data=url=s3://boot-images/{}/rootfs,etag=etag", new_image_id)
                    /* let aux = kernel_param
                        .trim_start_matches("nmd_data=url=s3://boot-images/")
                        .split_once('/')
                        .unwrap()
                        .1;

                    format!("nmd_data=url=s3://boot-images/{}/{}", new_image_id, aux) */
                } else {
                    kernel_param.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join(" "); */

            self.kernel = format!("s3://boot-images/{}/kernel", new_image_id);

            self.initrd = format!("s3://boot-images/{}/initrd", new_image_id);

            Ok(changed)
        }

        pub fn get_kernel_param_value(&self, key: &str) -> Option<String> {
            let params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            params.get(key).map(|value| value.to_string())
        }

        pub fn get_num_kernel_params(&self) -> usize {
            let params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            params.len()
        }

        /// Apply a str of kernel parameters:
        ///  - if kernel parameter already exists, then it will be updated
        ///  - if kernel parameters does not exists, then it will be deleted
        ///  - if kernel parameter does not exists, then it will be added
        /// Returns true if kernel params have change
        pub fn apply_kernel_params(&mut self, new_params: &str) -> bool {
            let mut change = false;

            let new_params: Vec<(&str, &str)> = new_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            for (new_key, new_value) in &new_params {
                for (key, value) in params.iter_mut() {
                    if *key == *new_key {
                        log::debug!("key '{}' found", key);
                        if value != new_value {
                            log::info!("changing key {} from {} to {}", key, value, new_value);

                            *value = new_value;
                            change = true
                        } else {
                            log::debug!("key '{}' value does not change ({})", key, value);
                        }
                    }
                }
            }

            if change == false {
                log::debug!("No value change in kernel params. Checking is either new params have been added or removed");
                if new_params.len() != params.len() {
                    log::info!("num kernel parameters have changed");
                    change = true;
                }
            }

            self.params = new_params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            change
        }

        /// Add a kernel parameter:
        ///  - if kernel parameter does not exists, then it will be added,
        /// otherwise nothing will change
        /// Returns true if kernel params have change
        pub fn add_kernel_param(&mut self, key: &str, new_value: &str) -> bool {
            let mut changed = false;
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            // NOTE: do not use --> `params.entry(key).or_insert(new_value);` otherwise, I don't know
            // how do we know if the key already exists or not
            if params.contains_key(key) {
                log::info!("key '{}' already exists, the new kernel parameter won't be added since it already exists", key);
                return changed;
            } else {
                log::info!(
                    "key '{}' not found, adding new kernel param with value '{}'",
                    key,
                    new_value
                );
                params.insert(key, new_value);
                changed = true
            }

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            changed
        }

        /* /// Apply kernel parameter. If kernel parameter already exists, then it will be updated,
        /// otherwise it will be added
        /// Input value expected the list of kernel parameters separated by space. eg: `console=ttyS0,115200 bad_page=panic crashkernel=512M hugepagelist=2m-2g intel_pstate=disable`
        pub fn upsert_kernel_params(&mut self, new_params: &str) {
            let new_params: Vec<(&str, &str)> = new_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            for (key, new_value) in new_params {
                params
                    .entry(key)
                    .and_modify(|value| *value = new_value)
                    .or_insert(new_value);
            }

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");
        }

        /// Apply kernel parameter. If kernel parameter already exists, then it will be updated,
        /// otherwise it will be added
        pub fn upsert_kernel_param(&mut self, key: &str, new_value: &str) {
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            params
                .entry(key)
                .and_modify(|value| *value = new_value)
                .or_insert(new_value);

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");
        } */

        /* /// Update kernel parameter. If kernel parameter exists, then it will be updated with new
        /// value. otherwise nothing will change
        /// Input value expected the list of kernel parameters separated by space. eg: `console=ttyS0,115200 bad_page=panic crashkernel=512M hugepagelist=2m-2g intel_pstate=disable`
        pub fn update_kernel_params(&mut self, new_params: &str) {
            let new_params: Vec<(&str, &str)> = new_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap())
                .collect();

            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            for (key, new_value) in new_params {
                params
                    .entry(key)
                    .and_modify(|value| *value = new_value)
                    .or_insert(new_value);
            }

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");
        } */

        /// Update kernel parameter. If kernel parameter exists, then it will be updated with new
        /// Note: This function won't make any change to params without values (eg: 'quiet') since
        /// they don't have values
        /// value. otherwise nothing will change
        pub fn update_kernel_param(&mut self, key: &str, new_value: &str) -> bool {
            let mut changed = false;
            // convert kernel params to a hashmap
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            // Update kernel param with new value
            // params.entry(key).and_modify(|value| *value = new_value);
            for (current_key, current_value) in params.iter_mut() {
                if *current_key == key {
                    *current_value = new_value;
                    changed = true;
                }
            }

            // Create new kernel params as a string
            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            changed
        }

        /* /// Delete kernel parameter. If kernel parameter exists, then it will be removed, otherwise
        /// nothing will be changed
        /// Input expected the list of kernel param keys separated by space. eg: `console bad_page crashkernel hugepagelist intel_pstate`
        pub fn delete_kernel_params(&mut self, keys: &str) {
            let keys: Vec<&str> = keys.split_whitespace().collect();

            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            for key in keys {
                params.remove(key);
            }

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");
        } */

        /// Delete kernel parameter. If kernel parameter exists, then it will be removed, otherwise
        /// nothing will be changed
        pub fn delete_kernel_param(&mut self, key: &str) -> bool {
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            let changed = params.remove(key).is_some();

            self.params = params
                .iter()
                .map(|(key, value)| {
                    if !value.is_empty() {
                        format!("{key}={value}")
                    } else {
                        key.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            changed
        }
    }

    pub mod http_client {

        use serde_json::Value;
        use tokio::sync::Semaphore;

        use core::result::Result;
        use std::{sync::Arc, time::Instant};

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
            let start = Instant::now();

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

                    get_raw(
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

            let duration = start.elapsed();
            log::info!("Time elapsed to get BSS bootparameters is: {:?}", duration);

            Ok(boot_params_vec)
        }

        /// Get node boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/get/
        pub async fn get_raw(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xnames: &[String],
        ) -> Result<Vec<BootParameters>, Error> {
            /* log::info!(
                "Get BSS bootparameters '{}'",
                if xnames.is_empty() {
                    "all available".to_string()
                } else {
                    xnames.join(",")
                }
            ); */

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
        use std::collections::HashMap;

        use serde_json::Value;

        use super::BootParameters;

        pub fn convert_kernel_params_to_map(kernel_params: &str) -> HashMap<String, String> {
            kernel_params
                .split_whitespace()
                .map(|kernel_param| {
                    let (key_str, value_str) =
                        kernel_param.split_once('=').unwrap_or((kernel_param, ""));

                    let key = key_str.to_string();
                    let value = value_str.to_string();

                    (key, value)
                })
                .collect()
        }

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
