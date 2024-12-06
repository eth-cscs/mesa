use infra_io;

use crate::{bss, common::authentication, error::Error, pcs};

pub struct Config {
    base_url: String,
    auth_token: String,
    root_cert: Vec<u8>,
    site_name: String,
}

impl Config {
    pub async fn new(
        base_url: &str,
        auth_token_opt: Option<&str>,
        root_cert: &[u8],
        site_name: &str,
    ) -> Result<Self, Error> {
        let auth_token = if let Some(auth_token) = auth_token_opt {
            auth_token.to_string()
        } else {
            let keycloak_base_url = base_url.to_string() + "/keycloak";

            authentication::get_api_token(base_url, root_cert, &keycloak_base_url, &site_name)
                .await?
        };

        Ok(Self {
            base_url: base_url.to_string(),
            auth_token: auth_token.to_string(),
            root_cert: root_cert.to_vec(),
            site_name: site_name.to_string(),
        })
    }
}

impl infra_io::contracts::Authentication for Config {
    async fn get_api_token(&self, _site_name: &str) -> Result<String, infra_io::error::Error> {
        let keycloak_base_url = self.base_url.clone() + "/keycloak";

        authentication::get_api_token(
            &self.base_url,
            &self.root_cert,
            &keycloak_base_url,
            &self.site_name,
        )
        .await
        .map_err(|e| infra_io::error::Error::Message(e.to_string()))
    }
}

impl infra_io::contracts::Power for Config {
    async fn power_off_sync(
        &self,
        nodes: &[String],
        force: bool,
    ) -> Result<serde_json::Value, infra_io::error::Error> {
        let operation = if force { "force-off" } else { "soft-off" };

        pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| infra_io::error::Error::Message(e.to_string()))
    }

    async fn power_on_sync(
        &self,
        nodes: &[String],
    ) -> Result<serde_json::Value, infra_io::error::Error> {
        let operation = "on";

        pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| infra_io::error::Error::Message(e.to_string()))
    }

    async fn power_reset_sync(
        &self,
        nodes: &[String],
        force: bool,
    ) -> Result<serde_json::Value, infra_io::error::Error> {
        let operation = if force {
            "hard-restart"
        } else {
            "soft-restart"
        };

        pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| infra_io::error::Error::Message(e.to_string()))
    }
}

impl infra_io::contracts::Boot for Config {
    async fn get_bootparameters(
        &self,
        nodes: &[String],
    ) -> Result<Vec<infra_io::types::BootParameters>, infra_io::error::Error> {
        let boot_parameter_vec =
            bss::http_client::get(&self.auth_token, &self.base_url, &self.root_cert, nodes)
                .await
                .map_err(|e| infra_io::error::Error::Message(e.to_string()))?;

        let mut boot_parameter_infra_vec = vec![];

        for boot_parameter in boot_parameter_vec {
            boot_parameter_infra_vec.push(infra_io::types::BootParameters {
                hosts: boot_parameter.hosts,
                macs: boot_parameter.macs,
                nids: boot_parameter.nids,
                params: boot_parameter.params,
                kernel: boot_parameter.kernel,
                initrd: boot_parameter.initrd,
                cloud_init: boot_parameter.cloud_init,
            });
        }

        Ok(boot_parameter_infra_vec)
    }

    async fn update_bootparameters(
        &self,
        boot_parameter: &infra_io::types::BootParameters,
    ) -> Result<Vec<serde_json::Value>, infra_io::error::Error> {
        let boot_parameters = bss::r#struct::BootParameters {
            hosts: boot_parameter.hosts.clone(),
            macs: boot_parameter.macs.clone(),
            nids: boot_parameter.nids.clone(),
            params: boot_parameter.params.clone(),
            kernel: boot_parameter.kernel.clone(),
            initrd: boot_parameter.initrd.clone(),
            cloud_init: boot_parameter.cloud_init.clone(),
        };

        bss::http_client::put(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            boot_parameters,
        )
        .await
        .map_err(|e| infra_io::error::Error::Message(e.to_string()))
    }
}
