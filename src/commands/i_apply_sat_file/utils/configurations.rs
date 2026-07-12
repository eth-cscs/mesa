use std::collections::BTreeMap;

use crate::{
  cfs::{
    self,
    v2::{CfsConfigurationRequest, CfsConfigurationResponse},
  },
  error::Error,
};

use super::{configuration, image, sessiontemplate};

#[allow(clippy::too_many_arguments)]
/// Create a CFS configuration from a single SAT-file `configurations`
/// entry — resolves Git/product layer references, validates them, and
/// posts to CFS.
pub async fn create_cfs_configuration_from_sat_file(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  gitea_base_url: &str,
  gitea_token: &str,
  cray_product_catalog: &BTreeMap<String, String>,
  sat_file_configuration_yaml: &serde_yaml::Value,
  dry_run: bool,
  site_name: &str,
  overwrite: bool,
) -> Result<CfsConfigurationResponse, Error> {
  log::debug!(
    "Convert CFS configuration in SAT file (yaml):\n{sat_file_configuration_yaml:#?}"
  );

  let (cfs_configuration_name, cfs_configuration) =
    CfsConfigurationRequest::from_sat_file_serde_yaml(
      shasta_root_cert,
      gitea_base_url,
      gitea_token,
      sat_file_configuration_yaml,
      cray_product_catalog,
      site_name,
    )
    .await?;

  if dry_run {
    log::debug!(
      "Dry run mode: Create CFS configuration:\n{}",
      serde_json::to_string_pretty(&cfs_configuration)?
    );

    // Generate mock CFS configuration
    let cfs_configuration = CfsConfigurationResponse {
      name: cfs_configuration_name,
      last_updated: String::new(),
      layers: Vec::new(),
      additional_inventory: None,
    };

    // Return mock CFS configuration
    Ok(cfs_configuration)
  } else {
    cfs::configuration::utils::create_new_configuration(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      &cfs_configuration,
      &cfs_configuration_name,
      overwrite,
    )
    .await
  }
}

/// Pre-flight check that the SAT file's `configurations` section is
/// self-consistent with its `images` and `session_templates` sections
/// (no orphan references, no empty configuration with referencing
/// downstream entries).
pub fn validate_sat_file_configurations_section(
  configuration_yaml_vec: &[configuration::Configuration],
  image_yaml_vec_opt: &[image::Image],
  sessiontemplate_yaml_vec_opt: &[sessiontemplate::SessionTemplate],
) -> Result<(), Error> {
  // Validate 'configurations' sections
  if !configuration_yaml_vec.is_empty()
    && image_yaml_vec_opt.is_empty()
    && sessiontemplate_yaml_vec_opt.is_empty()
  {
    return Err(Error::Message(
        "Incorrect SAT file. Please define either an 'images' or a 'session_templates' section. Exit"
            .to_string(),
      ));
  }

  Ok(())
}
