//! `BootParametersTrait` impl for [`super::Csm`].

use manta_backend_dispatcher::{
  error::Error, interfaces::bss::BootParametersTrait,
  types::bss::BootParameters as FrontEndBootParameters,
};

use super::Csm;

impl BootParametersTrait for Csm {
  async fn get_all_bootparameters(
    &self,
    auth_token: &str,
  ) -> Result<Vec<FrontEndBootParameters>, Error> {
    let boot_parameter_vec = self
      .shasta_client()
      .bss_bootparameters_get_all(auth_token)
      .await
      .map_err(Error::from)?;

    let boot_parameter_infra_vec = boot_parameter_vec
      .into_iter()
      .map(std::convert::Into::into)
      .collect();

    Ok(boot_parameter_infra_vec)
  }

  async fn get_bootparameters(
    &self,
    auth_token: &str,
    nodes: &[String],
  ) -> Result<Vec<FrontEndBootParameters>, Error> {
    let boot_parameter_vec = self
      .shasta_client()
      .bss_bootparameters_get_multiple(auth_token, nodes)
      .await
      .map_err(Error::from)?;

    let boot_parameter_infra_vec = boot_parameter_vec
      .into_iter()
      .map(std::convert::Into::into)
      .collect();

    Ok(boot_parameter_infra_vec)
  }

  async fn add_bootparameters(
    &self,
    auth_token: &str,
    boot_parameters: &FrontEndBootParameters,
  ) -> Result<(), Error> {
    self
      .shasta_client()
      .bss_bootparameters_post(auth_token, boot_parameters.clone().into())
      .await
      .map_err(Error::from)
  }

  async fn update_bootparameters(
    &self,
    auth_token: &str,
    boot_parameter: &FrontEndBootParameters,
  ) -> Result<(), Error> {
    self
      .shasta_client()
      .bss_bootparameters_patch(auth_token, &boot_parameter.clone().into())
      .await
      .map_err(Error::from)
  }

  async fn delete_bootparameters(
    &self,
    _auth_token: &str,
    _boot_parameters: &FrontEndBootParameters,
  ) -> Result<String, Error> {
    Err(Error::Message(
      "Delete boot parameters command not implemented for this backend"
        .to_string(),
    ))
  }
}
