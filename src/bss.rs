#[cfg(feature = "ochami")]
pub mod bootparameters {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use utils::get_image_id_from_s3_path;

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
        // FIXME: Change function signature so it returns a Result<String, Error> instead of String
        pub fn get_boot_image(&self) -> String {
            let params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| {
                    kernel_param
                        .split_once('=')
                        .map(|(key, value)| (key.trim(), value.trim()))
                        .unwrap_or((kernel_param, ""))
                })
                .collect();

            // NOTE: CN nodes have UIID image id in 'root' kernel parameter
            // Get `root` kernel parameter and split it by '/'
            let root_kernel_param_opt = params.get("root");
            // NOTE: CN nodes have UIID image id in 'metal.server' kernel parameter
            // Get `root` kernel parameter and split it by '/'
            let metal_server_kernel_param_opt = params.get("metal.server");

            let boot_image_id_opt: Option<&str> =
                if let Some(root_kernel_param) = root_kernel_param_opt {
                    get_image_id_from_s3_path(root_kernel_param)
                } else if let Some(metal_server_kernel_param) = metal_server_kernel_param_opt {
                    get_image_id_from_s3_path(metal_server_kernel_param)
                } else {
                    None
                };

            boot_image_id_opt.unwrap_or("").to_string()

            /* let mut path_elem_vec = self.kernel.split("/").skip(3);

            let mut image_id: String = path_elem_vec.next().unwrap_or_default().to_string();

            for path_elem in path_elem_vec {
                if !path_elem.eq("kernel") {
                    image_id = format!("{}/{}", image_id, path_elem);
                } else {
                    break;
                }
            }

            image_id */
        }

        /// Update boot image in kernel boot parameters and also in kernel and initrd fields if
        /// exists. Otherwise nothing is changed. This method updates both kernel params related to
        /// NCN and also CN
        /// Returns a boolean that indicates if kernel parameters have change:
        /// - kernel parameter value changed
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
                .map(|(key, value)| (key.trim(), value.trim()))
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

            self.kernel = format!("s3://boot-images/{}/kernel", new_image_id);

            self.initrd = format!("s3://boot-images/{}/initrd", new_image_id);

            Ok(changed)
        }

        pub fn get_kernel_param_value(&self, key: &str) -> Option<String> {
            let params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
                .collect();

            params.get(key).map(|value| value.to_string())
        }

        pub fn get_num_kernel_params(&self) -> usize {
            let params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
                .collect();

            params.len()
        }

        /// Apply a str of kernel parameters:
        ///  - current kernel params will be ignored/removed and replaced by the new ones
        /// Returns true if kernel params have change
        pub fn apply_kernel_params(&mut self, new_params: &str) -> bool {
            let mut change = false;

            let new_params: Vec<(&str, &str)> = new_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
                .collect();

            let mut params: HashMap<&str, &str> = HashMap::new();

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

        /// Set a str of kernel parameters:
        ///  - if kernel parameter already exists, then it will be updated
        ///  - if kernel parameter does not exists, then it will be added
        /// Returns true if kernel params have change
        pub fn update_kernel_params(&mut self, new_params: &str) -> bool {
            let mut change = false;

            let new_params: Vec<(&str, &str)> = new_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
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

            change
        }

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
                .map(|(key, value)| (key.trim(), value.trim()))
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

        /// Add a kernel parameter:
        ///  - if kernel parameter does not exists, then it will be added,
        /// otherwise nothing will change
        /// Returns true if kernel params have change
        pub fn add_kernel_params(&mut self, new_kernel_params: &str) -> bool {
            let mut changed = false;
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
                .collect();

            let new_kernel_params_tuple: HashMap<&str, &str> = new_kernel_params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .collect();

            for (key, new_value) in new_kernel_params_tuple {
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

        /// Delete kernel parameter. If kernel parameter exists, then it will be removed, otherwise
        /// nothing will be changed
        pub fn delete_kernel_params(&mut self, key: &str) -> bool {
            let mut params: HashMap<&str, &str> = self
                .params
                .split_whitespace()
                .map(|kernel_param| kernel_param.split_once('=').unwrap_or((kernel_param, "")))
                .map(|(key, value)| (key.trim(), value.trim()))
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

        pub fn post(
            base_url: &str,
            auth_token: &str,
            root_cert: &[u8],
            boot_parameters: BootParameters,
        ) -> Result<(), Error> {
            let client_builder = reqwest::blocking::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

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

            let api_url = format!("{}/boot/v1/bootparameters", base_url);

            let response = client
                .post(api_url)
                .bearer_auth(auth_token)
                .json(&boot_parameters)
                .send()
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(())
            } else {
                Err(Error::Message(response.text()?))
            }
        }

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
                    .text()
                    .await
                    .map_err(|error| Error::NetError(error))?;
                Err(Error::Message(payload))
            }
        }
    }

    pub mod utils {
        use std::collections::HashMap;

        use super::BootParameters;

        // Assumes s3 path looks like:
        // - s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/kernel
        // - craycps-s3:s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs:3dfae8d1fa3bb2bfb18152b4f9940ad0-667:dvs:api-gw-service-nmn.local:300:nmn0,hsn0:0
        // - url=s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs,etag=3dfae8d1fa3bb2bfb18152b4f9940ad0-667 bos_update_frequency=4h
        pub fn get_image_id_from_s3_path(s3_path: &str) -> Option<&str> {
            s3_path.split("/").skip(3).next()
        }

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

        /* /// Get Image ID from kernel field
        #[deprecated(
            since = "1.26.6",
            note = "Please convert from serde_json::Value to struct BootParameters use function `BootParameters::get_boot_image` instead"
        )]
        pub fn get_image_id(node_boot_params: &Value) -> String {
            serde_json::from_value::<BootParameters>(node_boot_params.clone())
                .unwrap()
                .get_boot_image()
        } */
    }
}

#[cfg(test)]
mod tests {
    use crate::bss::bootparameters::{utils::get_image_id_from_s3_path, BootParameters};

    #[test]
    fn test_get_image_id_from_s3_path() {
        assert_eq!(
            get_image_id_from_s3_path(
                "s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/kernel",
            ),
            Some("59e0180a-3fdd-4936-bba7-14ba914ffd34")
        );
    }

    #[test]
    fn test_get_image_id_from_s3_path_2() {
        assert_eq!(
            get_image_id_from_s3_path(
                "craycps-s3:s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs:3dfae8d1fa3bb2bfb18152b4f9940ad0-667:dvs:api-gw-service-nmn.local:300:nmn0,hsn0:0",
            ),
            Some("59e0180a-3fdd-4936-bba7-14ba914ffd34")
        );
    }

    #[test]
    fn test_get_image_id_from_s3_path_3() {
        assert_eq!(
            get_image_id_from_s3_path(
                "url=s3://boot-images/59e0180a-3fdd-4936-bba7-14ba914ffd34/rootfs,etag=3dfae8d1fa3bb2bfb18152b4f9940ad0-667 bos_update_frequency=4h",
            ),
            Some("59e0180a-3fdd-4936-bba7-14ba914ffd34")
        );
    }

    #[test]
    fn test_update_boot_image_ncn() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "ifname=mgmt0:14:02:ec:e3:cb:80 ifname=sun0:14:02:ec:e3:cb:81 ifname=mgmt1:b4:7a:f1:fe:63:16 ifname=sun1:b4:7a:f1:fe:63:17 biosdevname=1 pcie_ports=native transparent_hugepage=never console=tty0 console=ttyS0,115200 iommu=pt metal.server=s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/rootfs metal.no-wipe=1 ds=nocloud-net;s=http://10.92.100.81:8888/ rootfallback=LABEL=BOOTRAID initrd=initrd.img.xz root=live:LABEL=SQFSRAID rd.live.ram=0 rd.writable.fsimg=0 rd.skipfsck rd.live.overlay=LABEL=ROOTRAID rd.live.overlay.overlayfs=1 rd.luks rd.luks.crypttab=0 rd.lvm.conf=0 rd.lvm=1 rd.auto=1 rd.md=1 rd.dm=0 rd.neednet=0 rd.md.waitclean=1 rd.multipath=0 rd.md.conf=1 rd.bootif=0 hostname=ncn-s005 rd.net.timeout.carrier=120 rd.net.timeout.ifup=120 rd.net.timeout.iflink=120 rd.net.timeout.ipv6auto=0 rd.net.timeout.ipv6dad=0 append nosplash quiet crashkernel=360M log_buf_len=1 rd.retry=10 rd.shell ip=mgmt0:dhcp rd.peerdns=0 rd.net.dhcp.retry=5 psi=1 split_lock_detect=off rd.live.squashimg=rootfs rd.live.overlay.thin=0 rd.live.dir=1.5.0".to_string(),
            kernel: "s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/kernel".to_string(),
            initrd: "s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/initrd".to_string(),
            cloud_init: None,
        };

        let new_image_id = "my_new_image";

        let changed = boot_parameters.update_boot_image(new_image_id).unwrap();

        let mut pass = true;

        if !changed {
            pass = false;
            println!("DEBUG - pass 1 {}", pass);
        }

        for kernel_param in boot_parameters.params.split_whitespace() {
            if kernel_param.contains("metal.server=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
                println!("DEBUG - pass 2 {}", pass);
            }

            if kernel_param.contains("root=craycps-s3:s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
                println!("DEBUG - pass 3 {}", pass);
            }

            if kernel_param.contains("nmd_data=url=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
                println!("DEBUG - pass 4 {}", pass);
            }
        }

        pass = pass && boot_parameters.kernel.contains(new_image_id);
        println!("DEBUG - pass 5 {}", pass);
        pass = pass && boot_parameters.initrd.contains(new_image_id);
        println!("DEBUG - pass 6 {}", pass);

        assert!(pass)
    }

    #[test]
    fn test_update_boot_image_cn() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let new_image_id = "my_new_image";

        let changed = boot_parameters.update_boot_image(new_image_id).unwrap();

        let kernel_param_iter = boot_parameters.params.split_whitespace();

        let mut pass = true;

        for kernel_param in kernel_param_iter {
            if kernel_param.contains("metal.server=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("root=craycps-s3:s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("nmd_data=url=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }
        }

        pass = pass && boot_parameters.kernel.contains(new_image_id) && changed;
        pass = pass && boot_parameters.initrd.contains(new_image_id) && changed;

        assert!(pass)
    }

    #[test]
    fn test_add_kernel_param() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.add_kernel_params("test=1");

        let param_value_opt = boot_parameters.get_kernel_param_value("test");
        let new_num_params = boot_parameters.get_num_kernel_params();

        dbg!(&num_params);
        dbg!(&new_num_params);
        dbg!(&changed);
        dbg!(&param_value_opt);

        let pass = changed
            && (new_num_params == num_params + 1)
            && param_value_opt == Some("1".to_string());

        assert!(pass)
    }

    #[test]
    fn test_add_kernel_param_2() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.add_kernel_params("test=1 test2=2");

        let param_value_opt = boot_parameters.get_kernel_param_value("test");
        let param_value_2_opt = boot_parameters.get_kernel_param_value("test2");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value: {:?}", param_value_opt);

        let pass = changed
            && (new_num_params == num_params + 2)
            && param_value_opt == Some("1".to_string())
            && param_value_2_opt == Some("2".to_string());

        assert!(pass)
    }

    // Use apply_kernel_param function to remove 'quiet' kernel parameter
    #[test]
    fn test_apply_kernel_param() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.apply_kernel_params("console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f");

        let param_value_opt = boot_parameters.get_kernel_param_value("quiet");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);

        let pass = changed && (new_num_params == num_params - 1) && param_value_opt.is_none();

        assert!(pass)
    }

    // Use apply_kernel_param function to add 'test=1' kernel parameter
    #[test]
    fn test_apply_kernel_param_2() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.apply_kernel_params("console=ttyS0,115200 test=1 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f");

        let param_value_opt = boot_parameters.get_kernel_param_value("test");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed
            && (new_num_params == num_params + 1)
            && param_value_opt == Some("1".to_string());

        assert!(pass)
    }

    // Use apply_kernel_param function to add 2 kernel params 'test=1 test2=2'
    #[test]
    fn test_apply_kernel_param_3() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.apply_kernel_params("console=ttyS0,115200 test=1 bad_page=panic crashkernel=360M hugepagelist=2m-2g test2=2 intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f");

        let param_value_opt = boot_parameters.get_kernel_param_value("test");
        let param_value_2_opt = boot_parameters.get_kernel_param_value("test2");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed
            && (new_num_params == num_params + 2)
            && param_value_opt == Some("1".to_string())
            && param_value_2_opt == Some("2".to_string());

        assert!(pass)
    }

    // Use apply_kernel_param function to remove kernel param 'root'
    #[test]
    fn test_delete_kernel_param() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.delete_kernel_params("root");

        let param_value_opt = boot_parameters.get_kernel_param_value("root");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed && (new_num_params == num_params - 1) && param_value_opt == None;

        assert!(pass)
    }

    // Use delete_kernel_param function to remove all kernel params but 'root'
    #[test]
    fn test_delete_kernel_param_2() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.apply_kernel_params("root=test");

        let param_value_opt = boot_parameters.get_kernel_param_value("root");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed && (new_num_params == 1) && param_value_opt == Some("test".to_string());

        assert!(pass)
    }

    // Use set_kernel_param function to change value of kernel param 'root'
    #[test]
    fn test_set_kernel_param() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.update_kernel_params("root=test");

        let param_value_opt = boot_parameters.get_kernel_param_value("root");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed
            && (new_num_params == num_params)
            && param_value_opt == Some("test".to_string());

        assert!(pass)
    }

    // Use set_kernel_param function to change 2 kernel parameters 'root' and 'console'
    #[test]
    fn test_set_kernel_param_2() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.update_kernel_params("root=test console=test2");

        let param_value_opt = boot_parameters.get_kernel_param_value("root");
        let param_value_opt_2 = boot_parameters.get_kernel_param_value("console");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);
        println!("DEBUG - kernel param 2 value test: {:?}", param_value_opt_2);

        let pass = changed
            && (new_num_params == num_params)
            && param_value_opt == Some("test".to_string())
            && param_value_opt_2 == Some("test2".to_string());

        assert!(pass)
    }

    // Use set_kernel_param function to try to update a kernel param that does not exists. The end
    // result is that the original kernel params are not modified
    #[test]
    fn test_set_kernel_param_3() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let num_params = boot_parameters.get_num_kernel_params();

        let changed = boot_parameters.update_kernel_params("test=1");

        let param_value_opt = boot_parameters.get_kernel_param_value("test");
        let new_num_params = boot_parameters.get_num_kernel_params();

        println!("DEBUG - num kp: {}", num_params);
        println!("DEBUG - new num kp: {}", new_num_params);
        println!("DEBUG - changed: {}", changed);
        println!("DEBUG - kernel param value test: {:?}", param_value_opt);

        let pass = changed && (new_num_params == num_params) && param_value_opt == None;

        assert!(pass)
    }
}
