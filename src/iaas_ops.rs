use serde_json::Value;

use crate::error::Error;

pub trait IaaSOps {
    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_off_sync(&self, nodes: &Vec<String>, force: bool) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power off operation".to_string(),
        ))
    }

    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_on_sync(&self, nodes: &Vec<String>) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power on operation".to_string(),
        ))
    }

    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_reset_sync(&self, nodes: &Vec<String>, force: bool) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power reset operation".to_string(),
        ))
    }
}

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
    async fn power_off_sync(&self, nodes: &Vec<String>, force: bool) -> Result<Value, Error> {
        // Validate operation
        let operation = if force { "force-off" } else { "soft-off" };

        // Create power transition through CSM
        crate::pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes,
        )
        .await
    }

    async fn power_on_sync(&self, nodes: &Vec<String>) -> Result<Value, Error> {
        // Validate operation
        let operation = "on";

        // Create power transition through CSM
        crate::pcs::transitions::http_client::post_block(
            &self.base_url,
            &self.auth_token,
            &self.root_cert,
            operation,
            &nodes,
        )
        .await
    }

    async fn power_reset_sync(&self, nodes: &Vec<String>, force: bool) -> Result<Value, Error> {
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
            &nodes,
        )
        .await
    }
}
