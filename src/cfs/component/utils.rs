//! Helpers built on top of `ShastaClient::cfs_component_*` methods.

use crate::{cfs::component::http_client::v3::types::Component, error::Error};

/// PATCH a single CFS component to set its desired configuration and
/// enabled flag. Best-effort: failures are logged via the underlying
/// client but not returned.
pub async fn update_component_desired_configuration(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  xname: &str,
  desired_configuration: &str,
  enabled: bool,
) {
  let component = Component {
    id: Some(xname.to_string()),
    desired_config: Some(desired_configuration.to_string()),
    state: None,
    error_count: None,
    retry_policy: None,
    enabled: Some(enabled),
    tags: None,
    configuration_status: None,
    logs: None,
  };

  let Ok(client) = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  ) else {
    return;
  };

  let _ = client
    .cfs_component_v3_patch_component(shasta_token, component)
    .await;
}

/// PATCH the desired configuration and enabled flag on a list of CFS
/// components in one batch.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn update_component_list_desired_configuration(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  xnames: &[String],
  desired_configuration: &str,
  enabled: bool,
) -> Result<(), Error> {
  let mut component_list = Vec::new();

  for xname in xnames {
    let component = Component {
      id: Some(xname.clone()),
      desired_config: Some(desired_configuration.to_string()),
      state: None,
      error_count: None,
      retry_policy: None,
      enabled: Some(enabled),
      tags: None,
      configuration_status: None,
      logs: None,
    };

    component_list.push(component);
  }

  crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .cfs_component_v3_patch_component_list(shasta_token, component_list)
  .await?;

  Ok(())
}
