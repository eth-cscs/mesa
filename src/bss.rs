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
        return BootParameters {
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
            cloud_init: cloud_init_opt.map(|cloud_init| serde_json::to_value(cloud_init).unwrap()),
        };
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

    pub fn set_boot_image(&mut self, new_image: &str) {
        self.params = self
            .params
            .split_whitespace()
            .map(|boot_param| {
                if boot_param.contains("root=") {
                    let aux = boot_param
                        .trim_start_matches("root=craycps-s3:s3://boot-images/")
                        .split_once('/')
                        .unwrap()
                        .1;

                    format!("root=craycps-s3:s3://boot-images/{}/{}", new_image, aux)
                } else {
                    boot_param.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join(" ");

        self.kernel = format!("s3://boot-images/{}/kernel", new_image);

        self.kernel = format!("s3://boot-images/{}/kernel", new_image);
    }
}

pub mod http_client {

    use serde_json::Value;

    use std::error::Error;

    use core::result::Result;

    use super::BootParameters;

    /// Change nodes boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/put/
    pub async fn put(
        shasta_base_url: &str,
        shasta_token: &str,
        shasta_root_cert: &[u8],
        boot_parameters: BootParameters,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
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

        let resp = client
            .put(api_url)
            .json(&boot_parameters)
            // .json(&serde_json::json!({"hosts": xnames, "params": params, "kernel": kernel, "initrd": initrd})) // Encapsulating configuration.layers
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

    /// Get node boot params, ref --> https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/get/
    pub async fn get_boot_params(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        xnames: &[String],
    ) -> Result<Vec<BootParameters>, reqwest::Error> {
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

        let boot_param_vec = client
            .get(url_api)
            .query(&params)
            .bearer_auth(shasta_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<BootParameters>>()
            .await;

        // log::debug!("boot params:\n{:#?}", boot_param_vec);

        boot_param_vec
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
