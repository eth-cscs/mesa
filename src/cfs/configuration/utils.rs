//! Helpers built on top of `ShastaClient::cfs_configuration_*` methods.

use crate::{
  bos::{self, template::http_client::v2::types::BosSessionTemplate},
  cfs::{
    self, component::http_client::v2::types::Component,
    configuration::http_client::v2::types::cfs_configuration_response::CfsConfigurationResponse,
    session::http_client::v2::types::CfsSessionGetResponse,
  },
  common::{self, gitea},
  error::Error,
  hsm,
  ims::image::http_client::types::Image,
};

use chrono::NaiveDateTime;
use globset::Glob;
use serde_json::Value;

use super::http_client::{
  v2::types::cfs_configuration_request::CfsConfigurationRequest,
  v3::types::{
    cfs_configuration::LayerDetails, cfs_configuration_response::Layer,
  },
};

/// Create (or replace, when `overwrite=true`) a CFS v2 configuration
/// by name.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn create_new_configuration(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  configuration: &CfsConfigurationRequest,
  configuration_name: &str,
  overwrite: bool,
) -> Result<CfsConfigurationResponse, Error> {
  // Check if CFS configuration already exists
  log::debug!("Check CFS configuration '{configuration_name}' exists");

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let cfs_configuration_vec = shasta_client
    .cfs_configuration_v2_get(shasta_token, Some(configuration_name))
    .await
    .map_err(|e| Error::Message(e.to_string()))
    .unwrap_or_default();

  // Check if CFS configuration already exists and throw an error is that is the case
  if !cfs_configuration_vec.is_empty() {
    if overwrite {
      log::debug!(
        "CFS configuration '{configuration_name}' already exists but 'overwrite' has been enabled"
      );
    } else {
      log::warn!(
        "CFS configuration '{configuration_name}' already exists, cancel the process"
      );
      return Err(Error::ConfigurationAlreadyExists(
        configuration_name.to_string(),
      ));
    }
  }

  log::debug!(
    "CFS configuration '{configuration_name}' does not exists, creating new CFS configuration"
  );

  shasta_client
    .cfs_configuration_v2_put(
      shasta_token,
      &configuration.clone(),
      configuration_name,
    )
    .await
    .map_err(|e| Error::Message(e.to_string()))
}

/// Filter the list of CFS configurations provided. This operation is very expensive since it is
/// filtering by HSM group which means it needs to link CFS configurations with CFS sessions and
/// BOS sessiontemplate. Aditionally, it will also fetch CFS components to find CFS sessions and
/// BOS sessiontemplates linked to specific xnames that also belongs to the HSM group the user is
/// filtering from.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub fn filter(
  cfs_configuration_vec: &mut Vec<CfsConfigurationResponse>,
  xname_from_groups_vec: &[String],
  cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
  bos_sessiontemplate_vec: &mut Vec<BosSessionTemplate>,
  cfs_component_vec: &[Component],
  configuration_name_pattern_opt: Option<&str>,
  hsm_group_name_vec: &[String],
  since_opt: Option<NaiveDateTime>,
  until_opt: Option<NaiveDateTime>,
  limit_number_opt: Option<&u8>,
  keep_generic_sessions: bool,
) -> Result<Vec<CfsConfigurationResponse>, Error> {
  log::debug!("Filter CFS configurations");

  // Filter BOS sessiontemplates based on HSM groups
  let _ = bos::template::utils::filter(
    bos_sessiontemplate_vec,
    configuration_name_pattern_opt,
    hsm_group_name_vec,
    xname_from_groups_vec,
    None,
  );

  // Filter CFS sessions based on HSM groups
  cfs::session::utils::filter(
    cfs_session_vec,
    configuration_name_pattern_opt,
    hsm_group_name_vec,
    xname_from_groups_vec,
    None,
    None,
    keep_generic_sessions,
  )?;

  // Get boot image id and desired configuration from BOS sessiontemplates
  let image_id_cfs_configuration_target_from_bos_sessiontemplate: Vec<(
    String,
    String,
    Vec<String>,
  )> = bos::template::utils::get_image_id_cfs_configuration_target_tuple_vec(
    bos_sessiontemplate_vec,
  );

  // Get image id, configuration and targets from CFS sessions
  let image_id_cfs_configuration_target_from_cfs_session: Vec<(
    String,
    String,
    Vec<String>,
  )> = cfs::session::utils::get_image_id_cfs_configuration_target_tuple_vec(
    cfs_session_vec,
  );

  // Get desired configuration from CFS components
  let desired_config_vec: Vec<String> = cfs_component_vec
    .iter()
    .filter_map(|cfs_component| cfs_component.desired_config.clone())
    .collect();

  // Merge CFS configurations in list of filtered CFS sessions and BOS sessiontemplates and
  // desired configurations in CFS components
  let cfs_configuration_in_cfs_session_and_bos_sessiontemplate: Vec<String> = [
    image_id_cfs_configuration_target_from_bos_sessiontemplate
      .into_iter()
      .map(|(_, config, _)| config)
      .collect(),
    image_id_cfs_configuration_target_from_cfs_session
      .into_iter()
      .map(|(_, config, _)| config)
      .collect(),
    desired_config_vec,
  ]
  .concat();

  // Filter CFS configurations
  //
  // Filter CFS configurations based on HSM group names
  cfs_configuration_vec.retain(|cfs_configuration| {
    hsm_group_name_vec
      .iter()
      .any(|hsm_group| cfs_configuration.name.contains(hsm_group))
      || cfs_configuration_in_cfs_session_and_bos_sessiontemplate
        .contains(&cfs_configuration.name)
  });

  // Filter CFS configurations based on user input (date range or configuration name).
  // CFS configurations whose `last_updated` is missing or malformed can't be
  // confirmed in-range, so they're filtered out of the date-range view.
  if let (Some(since), Some(until)) = (since_opt, until_opt) {
    cfs_configuration_vec.retain(|cfs_configuration| {
      match chrono::DateTime::parse_from_rfc3339(
        &cfs_configuration.last_updated,
      ) {
        Ok(date) => {
          let date = date.naive_utc();
          since <= date && date < until
        }
        Err(e) => {
          log::warn!(
            "Skipping CFS configuration with unparseable last_updated '{}': {}",
            cfs_configuration.last_updated,
            e
          );
          false
        }
      }
    });
  }

  // Sort by last updated date in ASC order
  cfs_configuration_vec.sort_by(|cfs_configuration_1, cfs_configuration_2| {
    cfs_configuration_1
      .last_updated
      .cmp(&cfs_configuration_2.last_updated)
  });

  // Filter CFS configurations based on mattern matching
  if let Some(configuration_name_pattern) = configuration_name_pattern_opt {
    let glob = Glob::new(configuration_name_pattern)?.compile_matcher();
    cfs_configuration_vec.retain(|cfs_configuration| {
      glob.is_match(cfs_configuration.name.clone())
    });
  }

  // Limiting the number of results to return to client
  if let Some(limit_number) = limit_number_opt {
    *cfs_configuration_vec = cfs_configuration_vec[cfs_configuration_vec
      .len()
      .saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  Ok(cfs_configuration_vec.clone())
}

/// If filtering by HSM group, then configuration name must include HSM group name (It assumms each configuration
/// is built for a specific cluster based on ansible vars used by the CFS session). The reason
/// for this is because CSCS staff deletes all CFS sessions every now and then...
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn get_and_filter(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  configuration_name: Option<&str>,
  configuration_name_pattern: Option<&str>,
  hsm_group_name_vec: &[String],
  since_opt: Option<NaiveDateTime>,
  until_opt: Option<NaiveDateTime>,
  limit_number_opt: Option<&u8>,
) -> Result<Vec<CfsConfigurationResponse>, Error> {
  // Get list of configurations.
  // Returns list with only one element if "configuration name" provided

  // COLLECT SITE WIDE DATA FOR VALIDATION
  //
  let xname_from_groups_vec =
    hsm::group::utils::get_member_vec_from_hsm_name_vec(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      hsm_group_name_vec,
    )
    .await?;

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let (
    mut cfs_configuration_vec,
    mut cfs_session_vec,
    mut bos_sessiontemplate_vec,
    cfs_component_vec,
  ) = tokio::try_join!(
    shasta_client.cfs_configuration_v2_get(shasta_token, configuration_name),
    shasta_client.cfs_session_v2_get_all(shasta_token),
    shasta_client.bos_template_v2_get_all(shasta_token),
    shasta_client
      .cfs_component_v2_get_parallel(shasta_token, &xname_from_groups_vec),
  )?;

  let keep_generic_sessions = common::jwt_ops::is_user_admin(shasta_token);

  // Filter CFS configurations if user is not admin
  cfs::configuration::utils::filter(
    &mut cfs_configuration_vec,
    &xname_from_groups_vec,
    &mut cfs_session_vec,
    &mut bos_sessiontemplate_vec,
    &cfs_component_vec,
    configuration_name_pattern,
    hsm_group_name_vec,
    since_opt,
    until_opt,
    limit_number_opt,
    keep_generic_sessions,
  )?;

  Ok(cfs_configuration_vec)
}

/// Collect everything that references a CFS configuration: the CFS
/// sessions that ran against it, the IMS images it produced, and the
/// BOS session templates that consume those images.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_derivatives(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  configuration_name: &str,
) -> Result<
  (
    Option<Vec<CfsSessionGetResponse>>,
    Option<Vec<BosSessionTemplate>>,
    Option<Vec<Image>>,
  ),
  Error,
> {
  // List of image ids from CFS sessions and BOS sessiontemplates related to CFS configuration
  let mut image_id_vec: Vec<&str> = Vec::new();

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let (mut cfs_session_vec, mut bos_sessiontemplate_vec, mut ims_image_vec) = tokio::try_join!(
    shasta_client.cfs_session_v2_get_all(shasta_token),
    shasta_client.bos_template_v2_get_all(shasta_token),
    shasta_client.ims_image_get_all(shasta_token),
  )?;

  // Filter CFS sessions
  cfs::session::utils::filter_by_cofiguration(
    &mut cfs_session_vec,
    configuration_name,
  );

  // Filter BOS sessiontemplate
  bos_sessiontemplate_vec.retain(|bos_sessiontemplate| {
    bos_sessiontemplate
      .images_id()
      .any(|image_id_aux| image_id_vec.contains(&image_id_aux))
      || bos_sessiontemplate.get_configuration().unwrap_or_default()
        == configuration_name
  });

  // Add all image ids in CFS sessions into image_id_vec
  image_id_vec.extend(
    cfs_session_vec
      .iter()
      .flat_map(super::super::session::http_client::v2::types::CfsSessionGetResponse::results_id),
  );

  // Add boot images from BOS sessiontemplate to image_id_vec
  image_id_vec.extend(
    bos_sessiontemplate_vec
      .iter()
      .flat_map(crate::bos::template::http_client::v2::types::BosSessionTemplate::images_id),
  );

  // Filter images
  ims_image_vec.retain(|image| {
    image_id_vec.contains(&image.id.as_deref().unwrap_or_default())
  });

  Ok((
    Some(cfs_session_vec),
    Some(bos_sessiontemplate_vec),
    Some(ims_image_vec),
  ))
}

/// Resolve a CFS configuration layer to its detailed view by calling
/// Gitea for the layer's repo metadata (commit message, author, etc.).
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_configuration_layer_details(
  shasta_root_cert: &[u8],
  gitea_base_url: &str,
  gitea_token: &str,
  layer: Layer,
  site_name: &str,
) -> Result<LayerDetails, Error> {
  let commit_id: String =
    layer.commit.clone().unwrap_or("Not defined".to_string());
  let mut branch_name_vec: Vec<String> = Vec::new();
  let mut tag_name_vec: Vec<String> = Vec::new();
  let commit_sha;

  let repo_ref_vec_rslt = gitea::http_client::get_all_refs_from_repo_url(
    gitea_base_url,
    gitea_token,
    &layer.clone_url,
    shasta_root_cert,
  )
  .await;

  let repo_ref_vec = match repo_ref_vec_rslt {
    Ok(value) => value,
    Err(error) => {
      log::warn!(
        "Could not fetch repo '{}' refs. Reason:\n{:#?}",
        layer.clone_url,
        error
      );
      vec![]
    }
  };

  let mut ref_value_vec: Vec<&Value> = repo_ref_vec
    .iter()
    .filter(|repo_ref| {
      repo_ref
        .pointer("/object/sha")
        .eq(&Some(&Value::String(commit_id.clone())))
    })
    .collect();

  // Check if ref filtering returns an annotated tag, if so, then get the SHA of its
  // commit because it will be needed in case there are branches related to the
  // annotated tag
  //
  // NOTE: patterns matching array with single element

  if let [ref_value] = ref_value_vec.as_slice() {
    // Potentially an annotated tag
    log::debug!("Found ref in remote git repo:\n{ref_value:#?}");

    let ref_type: &str = ref_value
      .pointer("/object/type")
      .and_then(Value::as_str)
      .ok_or_else(|| {
        Error::GitRepoShape("ref type".to_string())
      })?;

    let mut ref_iter = ref_value
      .get("ref")
      .and_then(Value::as_str)
      .map(|v| v.split('/').skip(1))
      .ok_or_else(|| {
        Error::GitRepoShape("ref".to_string())
      })?;

    let ref_2 = ref_iter.nth(1);

    if ref_type == "tag" {
      // Yes, we are processing an annotated tag
      let tag_name = ref_2.ok_or_else(|| {
        Error::GitRepoShape("tag name".to_string())
      })?;

      let git_repo_tag_url = ref_value
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| {
          Error::GitRepoShape("tag url".to_string())
        })?;

      let commit_sha_value = gitea::http_client::get_commit_from_tag(
        git_repo_tag_url,
        tag_name,
        gitea_token,
        shasta_root_cert,
        site_name,
      )
      .await?;

      commit_sha = commit_sha_value
        .pointer("/commit/sha")
        .and_then(Value::as_str)
        .ok_or_else(|| {
          Error::GitRepoShape(
            "commit sha from git repo tag".to_string(),
          )
        })?;

      let annotated_tag_commit_sha =
        [commit_id.clone(), commit_sha.to_string()];

      ref_value_vec = repo_ref_vec
        .iter()
        .filter(|repo_ref| {
          repo_ref
            .pointer("/object/sha")
            .and_then(Value::as_str)
            .map(str::to_string)
            .is_some_and(|ref_sha| {
              annotated_tag_commit_sha.contains(&ref_sha)
            })
        })
        .collect();
    }
  }

  for ref_value in ref_value_vec {
    log::debug!("Found ref in remote git repo:\n{ref_value:#?}");
    let ref_type: &str = ref_value
      .pointer("/object/type")
      .and_then(Value::as_str)
      .ok_or_else(|| {
        Error::GitRepoShape("ref type".to_string())
      })?;
    let mut ref_iter = ref_value
      .get("ref")
      .and_then(Value::as_str)
      .map(|v| v.split('/').skip(1))
      .ok_or_else(|| {
        Error::GitRepoShape("ref".to_string())
      })?;

    let ref_1 = ref_iter.next();
    let ref_2 = ref_iter.collect::<Vec<_>>().join("/");

    if ref_type == "commit" {
      // either branch or lightweight tag
      if let (Some("heads"), branch_name_aux) = (ref_1, ref_2.clone()) {
        // branch
        branch_name_vec.push(branch_name_aux);
      } else if let (Some("tags"), tag_name_aux) = (ref_1, ref_2) {
        // lightweight tag
        tag_name_vec.push(tag_name_aux);
      }
    } else {
      // annotated tag
      tag_name_vec.push(ref_2);
    }
  }

  if let Some(cfs_config_layer_branch) = &layer.branch {
    branch_name_vec.push(cfs_config_layer_branch.clone());
  }

  let commit_id_opt = layer.commit.as_ref();

  let gitea_commit_details: serde_json::Value =
    if let Some(commit_id) = commit_id_opt {
      let repo_name = layer
        .clone_url
        .trim_start_matches("https://api-gw-service-nmn.local/vcs/")
        .trim_end_matches(".git");

      gitea::http_client::get_commit_details_from_external_url(
        repo_name,
        commit_id,
        gitea_token,
        shasta_root_cert,
        site_name,
      )
      .await?
    } else {
      serde_json::json!({})
    };

  Ok(LayerDetails::new(
    layer.name,
    layer
      .clone_url
      .trim_start_matches(
        format!("https://api.cmn.{site_name}.cscs.ch").as_str(),
      )
      .trim_end_matches(".git"),
    &commit_id,
    gitea_commit_details
      .pointer("/commit/committer/name")
      .and_then(Value::as_str)
      .unwrap_or("Not defined"),
    gitea_commit_details
      .pointer("/commit/committer/date")
      .and_then(Value::as_str)
      .unwrap_or("Not defined"),
    &branch_name_vec.join(","),
    &tag_name_vec.join(","),
    &layer.playbook,
  ))
}
