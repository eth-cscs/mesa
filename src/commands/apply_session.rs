//! Run a CFS session against a set of nodes.

use crate::{
  cfs::{
    self, v2::CfsSessionPostRequest, v3::CfsConfigurationRequest,
  },
  error::Error,
  node::utils::validate_xnames_format_and_membership_against_single_hsm,
};

use serde_json::Value;

/// Creates a CFS session target dynamic.
///
/// Returns a tuple like (`cfs configuration name`, `cfs session name`).
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn exec(
  client: &crate::ShastaClient,
  gitea_token: &str,
  gitea_base_url: &str,
  shasta_token: &str,
  cfs_conf_sess_name: Option<&str>,
  playbook_yaml_file_name_opt: Option<&str>,
  hsm_group: Option<&str>,
  repo_name_vec: &[&str],
  repo_last_commit_id_vec: &[&str],
  ansible_limit: Option<&str>,
  ansible_verbosity: Option<&str>,
  ansible_passthrough: Option<&str>,
  // watch_logs: bool,
) -> Result<(String, String), Error> {
  let shasta_base_url = client.base_url();
  let shasta_root_cert = client.root_cert();
  let mut xname_list: Vec<&str>;

  // Check andible limit matches the nodes in hsm_group
  let hsm_group_list;

  let hsm_groups_node_list;

  // * Validate input params
  // Neither hsm_group (both config file or cli arg) nor ansible_limit provided --> ERROR since we don't know the target nodes to apply the session to
  // NOTE: hsm group can be assigned either by config file or cli arg
  if ansible_limit.is_none() && hsm_group.is_none() && hsm_group.is_none() {
    return Err(Error::ApplySession("Need to specify either ansible-limit or hsm-group or both. (hsm-group value can be provided by cli param or in config file)".to_string()));
  }

  // * End validation input params

  // * Parse input params
  // Parse ansible limit
  // Get ansible limit nodes from cli arg
  let ansible_limit = ansible_limit.unwrap_or_default();
  let ansible_limit_nodes: Vec<&str> =
    ansible_limit.split(',').map(str::trim).collect();

  // Parse hsm group
  let mut hsm_group_value_opt = None;

  // Get hsm_group from cli arg
  if hsm_group.is_some() {
    hsm_group_value_opt = hsm_group;
  }
  // * End Parse input params

  let cfs_configuration_name = cfs_conf_sess_name.ok_or_else(|| {
    Error::ApplySession(
      "Error, --cfs-conf-sess-name argument is required.".to_string(),
    )
  })?;

  // * Process/validate hsm group value (and ansible limit)
  if let Some(hsm_group_value) = hsm_group_value_opt {
    // Get all hsm groups details related to hsm_group input
    hsm_group_list =
      crate::hsm::group::utils::get_members_for_groups_matching(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_value,
      )
      .await?;

    // Take all nodes for all hsm_groups found and put them in a Vec
    hsm_groups_node_list = hsm_group_list
      .iter()
      .flat_map(|hsm_group| {
        hsm_group.members.iter().map(std::string::String::as_str)
      })
      .collect();

    if ansible_limit_nodes.is_empty() {
      // hsm_group provided but no ansible_limit provided --> target nodes are the ones from hsm_group
      // included = hsm_groups_nodes
      xname_list = hsm_groups_node_list;
    } else {
      // both hsm_group provided and ansible_limit provided --> check ansible_limit belongs to hsm_group
      xname_list = hsm_groups_node_list;
      // Check user has provided valid XNAMES
      if let Ok(false) =
        validate_xnames_format_and_membership_against_single_hsm(
          shasta_token,
          shasta_base_url,
          shasta_root_cert,
          &xname_list,
          hsm_group,
        )
        .await
      {
        return Err(Error::ValidationFailed("xname/s invalid"));
      }
    }
  } else {
    // no hsm_group provided but ansible_limit provided --> target nodes are the ones from ansible_limit
    // included = ansible_limit_nodes
    xname_list = ansible_limit_nodes;
  }

  // * End Process/validate hsm group value (and ansible limit)

  // Remove duplicates in xname_list
  xname_list.sort_unstable();
  xname_list.dedup();

  log::debug!("Replacing '_' with '-' in repo name.");
  let cfs_configuration_name = str::replace(cfs_configuration_name, "_", "-");

  // * Check nodes are ready to run, create CFS configuration and CFS session
  let cfs_session_name =
    check_nodes_are_ready_to_run_cfs_configuration_and_run_cfs_session(
      client,
      &cfs_configuration_name,
      playbook_yaml_file_name_opt,
      repo_name_vec,
      repo_last_commit_id_vec,
      gitea_token,
      gitea_base_url,
      shasta_token,
      Some(&xname_list.join(",")), // Convert Hashset to String with comma separator, need to convert to Vec first following https://stackoverflow.com/a/47582249/1918003
      ansible_verbosity.map(|s| s.parse::<u8>().unwrap_or(2)),
      ansible_passthrough,
    )
    .await?;

  Ok((cfs_configuration_name, cfs_session_name))
}

/// Wait for the target nodes to be idle (no in-flight CFS session
/// targeting them), then launch a CFS session against them.
///
/// Pulls in the Gitea repo state (`repo_name_vec` /
/// `repo_last_commit_id_vec`) the configuration depends on, ensures
/// each xname in `limit` is ready, and then dispatches a new CFS
/// session for `cfs_configuration_name`. Useful as a pre-flight wrapper
/// around the lower-level CFS session APIs when nodes might still be
/// running a previous configuration.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn check_nodes_are_ready_to_run_cfs_configuration_and_run_cfs_session(
  client: &crate::ShastaClient,
  cfs_configuration_name: &str,
  playbook_yaml_file_name_opt: Option<&str>,
  repo_name_vec: &[&str],
  repo_last_commit_id_vec: &[&str],
  gitea_token: &str,
  gitea_base_url: &str,
  shasta_token: &str,
  limit: Option<&str>,
  ansible_verbosity: Option<u8>,
  ansible_passthrough: Option<&str>,
) -> Result<String, Error> {
  let shasta_base_url = client.base_url();
  let shasta_root_cert = client.root_cert();
  // Get ALL sessions
  let cfs_sessions = cfs::session::get_and_sort(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    None,
    None,
    None,
    None,
    None,
  )
  .await?;

  // FIXME: things to fix:
  //  - extend the list of nodes checked being modified byt also including those in CFS sessions
  //  working against HSM groups (and not just 'ansible limit')
  //  - get also the list of CFS sessions affecting those nodes
  let nodes_in_running_or_pending_cfs_session: Vec<&str> = cfs_sessions
    .iter()
    .filter(|cfs_session| {
      let status = cfs_session
        .status
        .as_ref()
        .and_then(|status| status.session.as_ref())
        .and_then(|session| session.status.as_ref())
        .map(String::as_str);
      status.is_some_and(|s| ["running", "pending"].contains(&s))
        && cfs_session
          .configuration
          .as_ref()
          .and_then(|configuration| configuration.name.as_ref())
          .map(String::as_str)
          == Some(cfs_configuration_name)
    })
    .flat_map(|cfs_session| {
      cfs_session
        .ansible
        .as_ref()
        .and_then(|ansible| ansible.limit.as_ref())
        .map(|ansible_limit| ansible_limit.split(','))
        .into_iter()
        .flatten()
    })
    .map(str::trim)
    .collect(); // TODO: remove duplicates... sort() + dedup() ???

  log::info!(
    "Nodes with cfs session running or pending: {nodes_in_running_or_pending_cfs_session:?}"
  );

  // NOTE: nodes can be a list of xnames or hsm group name

  // Convert limit (String with list of target nodes for new CFS session) into list of String
  let limit_value = limit.unwrap_or("");
  let nodes_list: Vec<&str> =
    limit_value.split(',').map(str::trim).collect();

  // Check each node if it has a CFS session already running
  for node in nodes_list {
    if nodes_in_running_or_pending_cfs_session.contains(&node) {
      return Err(Error::ApplySession(format!(
        "The node '{node}' from the list provided is already assigned to a running/pending CFS session. Please try again latter or delete the CFS session. Exitting"
      )));
    }
  }

  // Check nodes are ready to run a CFS layer
  let xnames: Vec<String> = limit_value
    .split(',')
    .map(|xname| String::from(xname.trim()))
    .collect();

  for xname in xnames {
    log::debug!("Checking status of component {xname}");

    let component_status = client
      .cfs_component_v2_get_single_component(shasta_token, &xname)
      .await?;

    let hsm_component_status_rslt = client
      .hsm_component_status_get(shasta_token, std::slice::from_ref(&xname))
      .await?;

    let hsm_component_status_state: &str = hsm_component_status_rslt
      .first()
      .and_then(|v| v.get("State"))
      .and_then(Value::as_str)
      .ok_or_else(|| {
        Error::ApplySession(format!(
          "HSM component status for '{xname}' is missing 'State'"
        ))
      })?;

    log::debug!(
      "HSM component state for component {xname}: {hsm_component_status_state}"
    );
    log::debug!(
      "Is component enabled for batched CFS: {:?}",
      component_status.enabled
    );
    log::debug!("Error count: {:?}", component_status.error_count);

    if hsm_component_status_state.eq("On")
      || hsm_component_status_state.eq("Standby")
    {
      return Err(Error::ApplySession("There is an CFS session scheduled to run on this node. Pleas try again later. Aborting".to_string()));
    }
  }

  let cfs_configuration = CfsConfigurationRequest::create_from_repos(
    gitea_token,
    gitea_base_url,
    shasta_root_cert,
    repo_name_vec,
    repo_last_commit_id_vec,
    playbook_yaml_file_name_opt,
  )
  .await?;

  // Update/PUT CFS configuration
  let cfs_configuration_name: String = client
    .cfs_configuration_v3_put(
      shasta_token,
      &cfs_configuration,
      cfs_configuration_name,
    )
    .await?
    .name;

  // Create dynamic CFS session
  let cfs_session_name = format!(
    "{}-{}",
    cfs_configuration_name,
    chrono::Utc::now().format("%Y%m%d%H%M%S")
  );

  let session = CfsSessionPostRequest::new(
    cfs_session_name,
    &cfs_configuration_name,
    limit,
    ansible_verbosity,
    ansible_passthrough,
    false,
    None,
    None,
  );

  let cfs_session_name = cfs::session::post(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &session,
  )
  .await?
  .name;

  Ok(cfs_session_name)
}
