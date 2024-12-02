use serde_json::Value;

use crate::{
    bss::{self, r#struct::BootParameters},
    error::Error,
    iaas_ops::IaaSOps,
};

pub struct Csm {
    pub base_url: String,
    pub auth_token: String,
    pub root_cert: Vec<u8>,
}

impl Csm {
    pub fn new(base_url: String, auth_token: String, root_cert: Vec<u8>) -> Csm {
        Csm {
            base_url,
            auth_token,
            root_cert,
        }
    }
}

impl IaaSOps for Csm {
    async fn power_off_sync(&self, nodes: &[String], force: bool) -> Result<Value, Error> {
        // Validate operation
        let operation = if force { "force-off" } else { "soft-off" };

        // Create power transition through CSM
        crate::pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(), // FIXME: change to slice
        )
        .await
    }

    async fn power_on_sync(&self, nodes: &[String]) -> Result<Value, Error> {
        // Validate operation
        let operation = "on";

        // Create power transition through CSM
        crate::pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(), // FIXME: change to slice
        )
        .await
    }

    async fn power_reset_sync(&self, nodes: &[String], force: bool) -> Result<Value, Error> {
        // Validate operation
        let operation = if force {
            "hard-restart"
        } else {
            "soft-restart"
        };

        // Create power transition through CSM
        crate::pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(), // FIXME: change to slice
        )
        .await
    }

    async fn get_bootparameters(&self, nodes: &[String]) -> Result<Vec<BootParameters>, Error> {
        bss::http_client::get(&self.auth_token, &self.base_url, &self.root_cert, nodes).await
    }

    async fn update_bootparameters(
        &self,
        boot_parameter: &BootParameters,
    ) -> Result<Vec<Value>, Error> {
        bss::http_client::patch(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            boot_parameter,
        )
        .await
    }
}
