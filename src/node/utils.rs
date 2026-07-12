//! Helpers built on top of the lower-level node-related APIs.
//!
//! Some validators here are exercised only by their own unit tests for
//! now; allow `dead_code` so the helpers stay available without breaking
//! non-test builds.
#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc, time::Instant};

use regex::Regex;
use tokio::sync::Semaphore;

use crate::{bss, cfs, error::Error, hsm};

use super::types::NodeDetails;

/// Validate user has access to a list of HSM group members provided.
/// HSM members user is asking for are taken from cli command
/// Exit if user does not have access to any of the members provided. By not having access to a HSM
/// members means, the node belongs to an HSM group which the user does not have access
pub async fn validate_target_hsm_members(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_group_members_opt: &[&str],
) -> Result<Vec<String>, Error> {
  let hsm_groups_user_has_access = hsm::group::utils::get_group_name_available(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
  )
  .await?;

  let xnames_user_has_access =
    hsm::group::utils::get_member_vec_from_hsm_name_vec(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      &hsm_groups_user_has_access,
    )
    .await?;

  // Check user has access to all xnames he is requesting
  if hsm_group_members_opt
    .iter()
    .all(|hsm_member| xnames_user_has_access.contains(&hsm_member.to_string()))
  {
    Ok(
      hsm_group_members_opt
        .as_ref()
        .iter()
        .copied()
        .map(str::to_string)
        .collect(),
    )
  } else {
    Err(Error::Message(format!(
      "Can't access all or any of the HSM members '{}'.\nPlease choose members form the list of HSM groups below:\n{}\nExit",
      hsm_group_members_opt.join(", "),
      hsm_groups_user_has_access.join(", ")
    )))
  }
}

/// Check if input is a NID
pub fn validate_nid_format_regex(node_vec: Vec<String>, regex: Regex) -> bool {
  node_vec.iter().all(|nid| regex.is_match(nid))
}

/// Check if input is a NID
pub fn validate_nid_format_vec(node_vec: Vec<String>) -> bool {
  node_vec.iter().all(|nid| validate_nid_format(nid))
}

/// Check if input is a NID. The check is case-insensitive: `nid000001`,
/// `NID000001`, and `Nid000001` are all valid.
pub fn validate_nid_format(nid: &str) -> bool {
  let lower = nid.to_lowercase();
  lower.len() == 9
    && lower
      .strip_prefix("nid")
      .is_some_and(|nid_number| nid_number.chars().all(char::is_numeric))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format_regex(
  node_vec: Vec<String>,
  regex: Regex,
) -> bool {
  node_vec.iter().all(|nid| regex.is_match(nid))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format_vec(node_vec: Vec<String>) -> bool {
  node_vec.iter().all(|nid| validate_xname_format(nid))
}

/// Validate xname is correct (it uses regex taken from HPE Cray CSM docs)
pub fn validate_xname_format(xname: &str) -> bool {
  let xname_re =
    Regex::new(r"^x\d{4}c[0-7]s([0-9]|[1-5][0-9]|6[0-4])b[0-1]n[0-7]$")
      .unwrap();

  xname_re.is_match(xname)
}

/// Validates a list of xnames.
/// Checks xnames strings are valid
/// If `hsm_group_name_opt` provided, then checks all xnames belongs to that `hsm_group`
// TODO: idually, we should create a struct with the data available to the user, then operate with
// it in memory, that way we avoid multiple calls to Shasta APIs
pub async fn validate_xnames_format_and_membership_against_single_hsm(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  xnames: &[&str],
  hsm_group_name_opt: Option<&str>,
) -> Result<bool, Error> {
  let hsm_group_members: Vec<String> =
    if let Some(hsm_group_name) = hsm_group_name_opt {
      hsm::group::utils::get_member_vec_from_hsm_group_name(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name,
      )
      .await?
    } else {
      Vec::new()
    };

  if xnames.iter().any(|&xname| {
    !validate_xname_format(xname)
      || (!hsm_group_members.is_empty()
        && !hsm_group_members.contains(&xname.to_string()))
  }) {
    return Ok(false);
  }

  Ok(true)
}

/// Fetch per-node component data for an arbitrary number of xnames by
/// batching requests.
///
/// CSM rejects requests that include too many xnames in a single call;
/// this helper chunks `xnames` and dispatches the batches concurrently.
pub async fn get_node_details(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  xname_list: Vec<String>,
) -> Result<Vec<NodeDetails>, Error> {
  let start = Instant::now();

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;

  let (
    components_status_rslt,
    node_boot_params_vec_rslt,
    node_hsm_info_rslt,
    cfs_session_vec_rslt,
  ) = tokio::join!(
    // Get CFS component status
    shasta_client.cfs_component_v2_get_multiple(shasta_token, &xname_list),
    // Get boot params to get the boot image id for each node
    shasta_client.bss_bootparameters_get_multiple(shasta_token, &xname_list),
    // Get HSM component status (needed to get NIDS)
    shasta_client.hsm_component_get_and_filter(shasta_token, &xname_list),
    // Get CFS sessions
    cfs::session::get_and_sort(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      None,
      None,
      None,
      None,
      Some(true),
    )
  );

  let node_hsm_info = node_hsm_info_rslt?;
  let node_boot_params_vec = node_boot_params_vec_rslt?;
  let cfs_session_vec = cfs_session_vec_rslt?;
  let components_status = components_status_rslt?;

  // ------------------------------------------------------------------------
  // Get and collect HSM members
  let mut node_details_map = HashMap::new();
  let mut tasks = tokio::task::JoinSet::new();

  let sem = Arc::new(Semaphore::new(10)); // CSM 1.3.1 higher number of concurrent tasks won't

  for xname in xname_list {
    let shasta_token_string = shasta_token.to_string();
    let shasta_base_url_string = shasta_base_url.to_string();
    let shasta_root_cert_vec = shasta_root_cert.to_vec();

    // find component details
    let component_details_opt = components_status
      .iter()
      .find(|component_status| component_status.id.as_ref().eq(&Some(&xname)));

    let Some(component_details) = component_details_opt else {
      return Err(Error::Message(format!(
        "ERROR - CFS component details for node {xname}"
      )));
    };

    let desired_configuration = &component_details.desired_config;
    let configuration_status = &component_details.configuration_status;
    let enabled = component_details.enabled;
    let error_count = component_details.error_count;

    // Get node HSM details. `Component100Component.id` is
    // `Option<XName100>`; compare via the inner `String` for parity
    // with the historical `Option<String>` shape.
    let node_hsm_info = node_hsm_info
      .iter()
      .find(|component| component.id.as_ref().map(|x| &x.0) == Some(&xname))
      .ok_or_else(|| Error::HsmComponentNotFound(xname.clone()))?;

    // `id` unwraps to an `XName100` reference; `.0` is the inner
    // `String`, and `.clone()` matches the historical owned-string
    // path.
    let node_hsm_id: String = node_hsm_info
      .id
      .as_ref()
      .ok_or_else(|| Error::HsmComponentIdNotDefined(xname.clone()))?
      .0
      .clone();

    // Get power status. `state` is now `Option<HmsState100>` (a `Copy`
    // enum with `Display` showing the wire name); `Display` already
    // emits the upper/mixed-case wire form (e.g. "Ready") so we keep
    // the historical uppercasing via `to_string().to_uppercase()`.
    let node_power_status = node_hsm_info
      .state
      .as_ref()
      .ok_or_else(|| Error::HsmComponentPowerStateNotDefined(xname.clone()))?
      .to_string()
      .to_uppercase();

    // Get NID. The OpenAPI schema declares NID as `type: integer`
    // (no `minimum: 0`), so progenitor picked `i64`; the
    // `HsmComponentNidNotDefined` error variant takes the xname string,
    // which is `node_hsm_id` (already an owned `String`).
    let nid = node_hsm_info
      .nid
      .ok_or_else(|| Error::HsmComponentNidNotDefined(node_hsm_id.clone()))?;

    // Calculate NID
    let node_nid = format!("nid{:0>6}", nid.to_string());

    // get node boot params (these are the boot params of the nodes with the image the node
    // boot with). the image in the bos sessiontemplate may be different i don't know why. need
    // to investigate
    let (image_id_in_kernel_params, kernel_params): (String, String) =
      if let Some(node_boot_params) =
        bss::utils::find_boot_params_related_to_node(
          &node_boot_params_vec,
          &xname,
        )
      {
        (node_boot_params.get_boot_image(), node_boot_params.params)
      } else {
        log::warn!("BSS boot parameters for node '{xname}' - NOT FOUND");
        ("Not found".to_string(), "Not found".to_string())
      };

    // Get CFS configuration related to image id
    let cfs_session_related_to_image_id_opt =
      cfs::session::utils::find_cfs_session_related_to_image_id(
        &cfs_session_vec,
        &image_id_in_kernel_params,
      );

    let cfs_configuration_boot = if let Some(cfs_session_related_to_image_id) =
      cfs_session_related_to_image_id_opt
    {
      let session_name = cfs_session_related_to_image_id.name;

      cfs_session_related_to_image_id
        .configuration
        .ok_or_else(|| {
          Error::SessionConfigurationNotDefined(session_name.clone())
        })?
        .name
        .ok_or_else(|| {
          Error::SessionConfigurationNotDefined(session_name.clone())
        })?
    } else {
      "Not found".to_string()
    };

    // CFS component fields are all optional on the wire (a node may
    // have no assigned configuration, no recorded state, etc.). Fall
    // back to the "Not found" sentinel used elsewhere in this function
    // rather than panicking on None.
    let desired_configuration_str = desired_configuration
      .clone()
      .unwrap_or_else(|| "Not found".to_string());
    let configuration_status_str = configuration_status
      .clone()
      .unwrap_or_else(|| "Not found".to_string());
    let enabled_str = enabled
      .as_ref().map_or_else(|| "Not found".to_string(), bool::to_string);
    let error_count_str = error_count
      .as_ref().map_or_else(|| "Not found".to_string(), u64::to_string);

    node_details_map
      .entry(xname.clone())
      .and_modify(|node_details: &mut NodeDetails| {
        node_details.xname = xname.clone();
        node_details.nid = node_nid.clone();
        node_details.hsm = String::new();
        node_details.power_status = node_power_status.clone();
        node_details.desired_configuration = desired_configuration_str.clone();
        node_details.configuration_status = configuration_status_str.clone();
        node_details.enabled = enabled_str.clone();
        node_details.error_count = error_count_str.clone();
        node_details.boot_image_id = image_id_in_kernel_params.clone();
        node_details.boot_configuration = cfs_configuration_boot.clone();
        node_details.kernel_params = kernel_params.clone();
      })
      .or_insert(NodeDetails {
        xname: xname.clone(),
        nid: node_nid,
        hsm: String::new(),
        power_status: node_power_status,
        desired_configuration: desired_configuration_str,
        configuration_status: configuration_status_str,
        enabled: enabled_str,
        error_count: error_count_str,
        boot_image_id: image_id_in_kernel_params,
        boot_configuration: cfs_configuration_boot,
        kernel_params,
      });

    let permit = Arc::clone(&sem).acquire_owned().await;

    tasks.spawn(async move {
      let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

      crate::ShastaClient::new(
        &shasta_base_url_string,
        shasta_root_cert_vec.clone(),
      )?
      .hsm_memberships_get_xname(&shasta_token_string, &xname)
      .await
    });
  }

  while let Some(message) = tasks.join_next().await {
    let node_membership = message??;

    let node_details = NodeDetails {
      xname: String::new(),
      nid: String::new(),
      hsm: node_membership.group_labels.join(", "),
      power_status: String::new(),
      desired_configuration: String::new(),
      configuration_status: String::new(),
      enabled: String::new(),
      error_count: String::new(),
      boot_image_id: String::new(),
      boot_configuration: String::new(),
      kernel_params: String::new(),
    };

    let membership_id = node_membership
      .id
      .as_ref()
      .map(|x| x.0.clone())
      .unwrap_or_default();
    node_details_map
      .entry(membership_id)
      .and_modify(|node_details: &mut NodeDetails| {
        node_details.hsm = node_membership.group_labels.join(", ");
      })
      .or_insert(node_details);
  }

  let duration = start.elapsed();
  log::debug!("Time elapsed to get node details is: {duration:?}");
  // ------------------------------------------------------------------------

  Ok(node_details_map.into_values().collect())
}


#[cfg(test)]
mod tests {
  use super::*;

  // ---------- validate_nid_format ----------

  #[test]
  fn validate_nid_format_accepts_canonical_nid() {
    assert!(validate_nid_format("nid000001"));
    assert!(validate_nid_format("nid123456"));
  }

  #[test]
  fn validate_nid_format_is_case_insensitive() {
    assert!(validate_nid_format("NID000001"));
    assert!(validate_nid_format("Nid000001"));
    assert!(validate_nid_format("nID000001"));
  }

  #[test]
  fn validate_nid_format_rejects_wrong_length() {
    assert!(!validate_nid_format("nid00001")); // 8 chars
    assert!(!validate_nid_format("nid0000001")); // 10 chars
    assert!(!validate_nid_format(""));
  }

  #[test]
  fn validate_nid_format_rejects_missing_prefix() {
    assert!(!validate_nid_format("000000001"));
    assert!(!validate_nid_format("xyz000001"));
  }

  #[test]
  fn validate_nid_format_rejects_non_numeric_suffix() {
    assert!(!validate_nid_format("nid0000ab"));
    assert!(!validate_nid_format("nid-00001"));
  }

  #[test]
  fn validate_nid_format_vec_all_or_nothing() {
    assert!(validate_nid_format_vec(vec![
      "nid000001".into(),
      "nid000002".into(),
    ]));
    assert!(!validate_nid_format_vec(vec![
      "nid000001".into(),
      "not-a-nid".into(),
    ]));
    assert!(validate_nid_format_vec(vec![])); // vacuous: all() of empty is true
  }

  // ---------- validate_xname_format ----------

  #[test]
  fn validate_xname_format_accepts_canonical_xname() {
    assert!(validate_xname_format("x1000c0s0b0n0"));
    assert!(validate_xname_format("x9999c7s64b1n7"));
  }

  #[test]
  fn validate_xname_format_rejects_out_of_range_components() {
    // c must be 0..=7
    assert!(!validate_xname_format("x1000c8s0b0n0"));
    // s must be 0..=64
    assert!(!validate_xname_format("x1000c0s65b0n0"));
    // b must be 0..=1
    assert!(!validate_xname_format("x1000c0s0b2n0"));
    // n must be 0..=7
    assert!(!validate_xname_format("x1000c0s0b0n8"));
  }

  #[test]
  fn validate_xname_format_rejects_missing_or_extra_parts() {
    assert!(!validate_xname_format(""));
    assert!(!validate_xname_format("x1000"));
    assert!(!validate_xname_format("x1000c0s0b0n0extra"));
    assert!(!validate_xname_format("not-an-xname"));
  }

  #[test]
  fn validate_xname_format_requires_four_digit_cabinet() {
    assert!(!validate_xname_format("x100c0s0b0n0")); // 3 digits
    assert!(!validate_xname_format("x10000c0s0b0n0")); // 5 digits
  }

  #[test]
  fn validate_xname_format_vec_all_or_nothing() {
    assert!(validate_xname_format_vec(vec![
      "x1000c0s0b0n0".into(),
      "x1000c0s0b0n1".into(),
    ]));
    assert!(!validate_xname_format_vec(vec![
      "x1000c0s0b0n0".into(),
      "garbage".into(),
    ]));
    assert!(validate_xname_format_vec(vec![]));
  }
}
