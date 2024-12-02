use serde_json::Value;

use crate::{bss::r#struct::BootParameters, csm::Csm, error::Error};

pub trait IaaSOps {
    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_off_sync(&self, _nodes: &[String], _force: bool) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power off operation".to_string(),
        ))
    }

    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_on_sync(&self, _nodes: &[String]) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power on operation".to_string(),
        ))
    }

    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    async fn power_reset_sync(&self, _nodes: &[String], _force: bool) -> Result<Value, Error> {
        Err(Error::Message(
            "This infrastructure does not support power reset operation".to_string(),
        ))
    }

    async fn get_bootparameters(&self, _nodes: &[String]) -> Result<Vec<BootParameters>, Error> {
        Err(Error::Message(
            "This infrastructure does not support update boot parameters operation".to_string(),
        ))
    }

    async fn update_bootparameters(
        &self,
        _boot_parameter: &BootParameters,
    ) -> Result<Vec<Value>, Error> {
        Err(Error::Message(
            "This infrastructure does not support update boot parameters operation".to_string(),
        ))
    }
}

pub fn new_iaas(
    iaas_name: &str,
    base_url: String,
    auth_token: String,
    root_cert: Vec<u8>,
) -> Result<impl IaaSOps, Error> {
    match iaas_name {
        "csm" => Ok(Csm::new(base_url, auth_token, root_cert)), // FIXME: check if it is possible
        // to not have the IaaS name hardcoded here and just have the 'new' function in the IaaS
        // crate
        _ => Err(Error::Message(format!(
            "ERROR - infrastructure '{}' is not valid",
            iaas_name
        ))),
    }
}
