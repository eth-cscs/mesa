//! `PCSTrait` impl for [`super::Csm`].

use manta_backend_dispatcher::{
  error::Error,
  interfaces::pcs::PCSTrait,
  types::pcs::{
    power_status::types::PowerStatusAll as FrontEndPowerStatusAll,
    transitions::types::{TransitionResponse, TransitionStartOutput},
  },
};

use super::Csm;

impl PCSTrait for Csm {
  async fn pcs_transitions_post(
    &self,
    auth_token: &str,
    operation: &str,
    nodes: &[String],
  ) -> Result<TransitionStartOutput, Error> {
    self
      .shasta_client()
      .pcs_transitions_post(auth_token, operation, nodes)
      .await
      .map(Into::into)
      .map_err(Error::from)
  }

  async fn pcs_transitions_get(
    &self,
    auth_token: &str,
    transition_id: &str,
  ) -> Result<TransitionResponse, Error> {
    self
      .shasta_client()
      .pcs_transitions_get_by_id(auth_token, transition_id)
      .await
      .map(Into::into)
      .map_err(Error::from)
  }

  async fn power_status(
    &self,
    auth_token: &str,
    nodes: &[String],
    power_state_filter: Option<&str>,
    management_state_filter: Option<&str>,
  ) -> Result<FrontEndPowerStatusAll, Error> {
    let nodes_str: Vec<&str> = nodes.iter().map(String::as_str).collect();

    self
      .shasta_client()
      .pcs_power_status_post(
        auth_token,
        Some(nodes_str.as_slice()),
        power_state_filter,
        management_state_filter,
      )
      .await
      .map(|status| {
        log::info!("return value from async fn power_status : {status:?}");
        status.into()
      })
      .map_err(Error::from)
  }
}
