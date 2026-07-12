//! Entry-point function for the apply-hw-cluster-pin workflow.

use std::collections::HashMap;

use crate::{
  commands::apply_hw_cluster_pin::utils::{
    calculate_hsm_hw_component_summary, get_hsm_node_hw_component_counter,
    resolve_hw_description_to_xnames,
  },
  error::Error,
  hsm::{self, group::types::Group},
};

/// Apply a hardware pattern to (re)compose an HSM group from a parent
/// group.
///
/// Computes the set of xnames in `parent_hsm_group_name` whose hardware
/// matches `pattern` (e.g. `"a100:gpu=4"`), then moves that set into
/// `target_hsm_group_name`, optionally creating the target if it
/// doesn't exist and removing the parent once it's empty.
///
/// # Arguments
///
/// - `target_hsm_group_name` — destination group.
/// - `parent_hsm_group_name` — source group to draw nodes from.
/// - `pattern` — `key:value`-style hardware filter.
/// - `nodryrun` — when `false`, the function only logs the intended
///   changes without mutating CSM.
/// - `create_target_hsm_group` — create the target group if it doesn't
///   exist.
/// - `delete_empty_parent_hsm_group` — delete the parent if it ends up
///   with zero members.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn exec(
  client: &crate::ShastaClient,
  shasta_token: &str,
  target_hsm_group_name: &str,
  parent_hsm_group_name: &str,
  pattern: &str,
  nodryrun: bool,
  create_target_hsm_group: bool,
  delete_empty_parent_hsm_group: bool,
) -> Result<(), Error> {
  let shasta_base_url = client.base_url();
  let shasta_root_cert = client.root_cert();
  // *********************************************************************************************************
  // PREPREQUISITES - FORMAT USER INPUT

  let pattern = format!("{target_hsm_group_name}:{pattern}");

  log::info!("pattern: {pattern}");

  // lcm -> used to normalize and quantify memory capacity
  let mem_lcm = 16384; // 1024 * 16

  // Normalize text in lowercase and separate each HSM group hw inventory pattern
  let pattern_lowercase = pattern.to_lowercase();

  // pattern was constructed above with a ':' between target_hsm_group_name
  // and the user-supplied pattern, so split_once cannot fail here.
  let (target_hsm_group_name, pattern_hw_component) = pattern_lowercase
    .split_once(':')
    .expect("pattern built with ':' separator above");

  let pattern_element_vec: Vec<&str> =
    pattern_hw_component.split(':').collect();

  let mut user_defined_target_hsm_hw_component_count_hashmap: HashMap<
    String,
    usize,
  > = HashMap::new();

  // Check user input is correct
  for hw_component_counter in pattern_element_vec.chunks(2) {
    if hw_component_counter.len() < 2 {
      return Err(Error::ValidationFailed("Error in pattern. Please make sure to follow <hsm name>:<hw component>:<counter>:... eg tasna:a100:4:epyc:10:instinct:8"));
    }
    let count = hw_component_counter[1].parse::<usize>().map_err(|_| {
      Error::ApplySession(format!(
        "Error in pattern: '{}' is not a valid integer count",
        hw_component_counter[1]
      ))
    })?;
    user_defined_target_hsm_hw_component_count_hashmap
      .insert(hw_component_counter[0].to_string(), count);
  }

  log::info!(
    "User defined hw components with counters: {user_defined_target_hsm_hw_component_count_hashmap:?}"
  );

  let mut user_defined_target_hsm_hw_component_vec: Vec<String> =
    user_defined_target_hsm_hw_component_count_hashmap
      .keys()
      .cloned()
      .collect();

  user_defined_target_hsm_hw_component_vec.sort();

  // *********************************************************************************************************
  // PREPREQUISITES - GET DATA - TARGET HSM

  let shasta_client = client;
  match shasta_client
    .hsm_group_get(
      shasta_token,
      Some(&[target_hsm_group_name.to_string()]),
      None,
    )
    .await
  {
    Ok(_) => {
      log::debug!("Target HSM group {target_hsm_group_name} exists, good.");
    }
    Err(_) => {
      if create_target_hsm_group {
        log::info!(
          "Target HSM group {target_hsm_group_name} does not exist, but the option to create the group has been selected, creating it now."
        );
        if nodryrun {
          // Post-progenitor field shapes: `label` and `exclusive_group`
          // wrap a `String` in `ResourceName(pub String)`; `tags` is
          // a `Vec<ResourceName>` (no `Option`).
          use crate::hsm::group::types::ResourceName;
          let group = Group {
            label: ResourceName(target_hsm_group_name.to_string()),
            description: None,
            tags: vec![],
            members: None,
            exclusive_group: Some(ResourceName("false".to_string())),
          };

          let _ = shasta_client.hsm_group_post(shasta_token, group).await?;
        } else {
          return Err(Error::ValidationFailed(
            "Dryrun selected, cannot create the new group and continue.",
          ));
        }
      } else {
        return Err(Error::ApplySession(format!(
          "Target HSM group {target_hsm_group_name} does not exist, but the option to create the group was NOT specificied, cannot continue."
        )));
      }
    }
  }

  // Get target HSM group members
  let target_hsm_group_member_vec: Vec<String> =
    hsm::group::utils::get_member_vec_from_hsm_name_vec(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      &[target_hsm_group_name.to_string()],
    )
    .await?;

  // Get HSM hw component counters for target HSM
  let mut target_hsm_node_hw_component_count_vec: Vec<(
    String,
    HashMap<String, usize>,
  )> = get_hsm_node_hw_component_counter(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &user_defined_target_hsm_hw_component_vec,
    &target_hsm_group_member_vec,
    mem_lcm,
  )
  .await?;

  // Sort nodes hw counters by node name
  target_hsm_node_hw_component_count_vec.sort_by_key(
    |target_hsm_group_hw_component| target_hsm_group_hw_component.0.clone(),
  );

  // Calculate hw component counters (summary) across all node within the HSM group
  let target_hsm_hw_component_summary_hashmap: HashMap<String, usize> =
    calculate_hsm_hw_component_summary(&target_hsm_node_hw_component_count_vec);

  log::debug!(
    "HSM group '{target_hsm_group_name}' hw component summary: {target_hsm_hw_component_summary_hashmap:?}"
  );

  // *********************************************************************************************************
  // PREPREQUISITES - GET DATA - PARENT HSM

  // Get target HSM group members
  let parent_hsm_group_member_vec: Vec<String> =
    hsm::group::utils::get_member_vec_from_hsm_name_vec(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      &[parent_hsm_group_name.to_string()],
    )
    .await?;

  // Get HSM hw component counters for parent HSM
  let mut parent_hsm_node_hw_component_count_vec: Vec<(
    String,
    HashMap<String, usize>,
  )> = get_hsm_node_hw_component_counter(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &user_defined_target_hsm_hw_component_vec,
    &parent_hsm_group_member_vec,
    mem_lcm,
  )
  .await?;

  // Sort nodes hw counters by node name
  parent_hsm_node_hw_component_count_vec.sort_by_key(
    |parent_hsm_group_hw_component| parent_hsm_group_hw_component.0.clone(),
  );

  // *********************************************************************************************************
  // VALIDATE USER INPUT - CHECK HARDWARE REQUIREMENTS REQUESTED BY USER CAN BE FULFILLED
  // CHECK USER HAS ACCESS TO REQUESTED HW COMPONENTS
  // CHECK USER HAS ACCESS TO ENOUGH QUANTITY OF HW RESOURCES REQUESTED

  let mut combined_target_parent_hsm_node_hw_component_count_vec =
    parent_hsm_node_hw_component_count_vec.clone();

  for elem in &target_hsm_node_hw_component_count_vec {
    if !parent_hsm_node_hw_component_count_vec
      .iter()
      .any(|(xname, _)| xname.eq(&elem.0))
    {
      combined_target_parent_hsm_node_hw_component_count_vec.push(elem.clone());
    }
  }

  let combined_target_parent_hsm_hw_component_summary_hashmap =
    calculate_hsm_hw_component_summary(
      &combined_target_parent_hsm_node_hw_component_count_vec,
    );

  for (hw_component, qty) in &user_defined_target_hsm_hw_component_count_hashmap
  {
    if combined_target_parent_hsm_hw_component_summary_hashmap
      .get(hw_component)
      .is_some_and(|value| value >= qty)
    {
      // We are ok, user has access to enough resources to fullfill its request
    } else {
      // There are not enough resources to fulfill the user request
      return Err(Error::ValidationFailed(
        "There are not enough resources to fulfill user request.",
      ));
    }
  }

  // *********************************************************************************************************
  // CONVERT THE HARDWARE DESCRIPTION INTO A SET OF NODES IN TARGET HSM

  let (
    target_hsm_node_hw_component_count_vec,
    parent_hsm_node_hw_component_count_vec,
  ) = resolve_hw_description_to_xnames(
    target_hsm_node_hw_component_count_vec,
    parent_hsm_node_hw_component_count_vec,
    user_defined_target_hsm_hw_component_count_hashmap,
  )?;

  // Calculate hw component counters (summary) across all node within the HSM group
  let target_hsm_hw_component_summary_hashmap =
    calculate_hsm_hw_component_summary(&target_hsm_node_hw_component_count_vec);

  // Calculate hw component counters (summary) across all node within the HSM group
  let parent_hsm_hw_component_summary_hashmap =
    calculate_hsm_hw_component_summary(&parent_hsm_node_hw_component_count_vec);

  let target_hsm_node_vec = target_hsm_node_hw_component_count_vec
    .into_iter()
    .map(|(xname, _)| xname)
    .collect::<Vec<String>>();

  let parent_hsm_node_vec = parent_hsm_node_hw_component_count_vec
    .into_iter()
    .map(|(xname, _)| xname)
    .collect::<Vec<String>>();

  // *********************************************************************************************************
  // UPDATE TARGET HSM GROUP IN CSM
  log::info!(
    "Updating target HSM group '{target_hsm_group_name}' members"
  );
  if nodryrun {
    // The target HSM group will never be empty, the way the pattern works it'll always
    // contain at least one node, so there is no need to add code to delete it if it's empty.
    let _ = hsm::group::utils::update_hsm_group_members(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      target_hsm_group_name,
      &target_hsm_group_member_vec
        .iter()
        .map(String::as_str)
        .collect::<Vec<&str>>(),
      &target_hsm_node_vec
        .iter()
        .map(String::as_str)
        .collect::<Vec<&str>>(),
    )
    .await;
  } else {
    log::info!("Dry run enabled, not modifying the HSM groups on the system.");
  }

  // *********************************************************************************************************
  // UPDATE PARENT GROUP IN CSM
  log::info!(
    "Updating parent HSM group '{parent_hsm_group_name}' members"
  );
  if nodryrun {
    // The parent group might be out of resources after applying this, so it's safe to check
    // if there are still nodes there and, delete it after moving out the resources.
    let parent_group_will_be_empty =
      target_hsm_group_member_vec.len() == parent_hsm_group_member_vec.len();
    let _ = hsm::group::utils::update_hsm_group_members(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      parent_hsm_group_name,
      &parent_hsm_group_member_vec
        .iter()
        .map(String::as_str)
        .collect::<Vec<&str>>(),
      &parent_hsm_node_vec
        .iter()
        .map(String::as_str)
        .collect::<Vec<&str>>(),
    )
    .await;
    if parent_group_will_be_empty {
      if delete_empty_parent_hsm_group {
        log::info!(
          "Parent HSM group {parent_hsm_group_name} is now empty and the option to delete empty groups has been selected, removing it."
        );
        match shasta_client
          .hsm_group_delete_group(shasta_token, parent_hsm_group_name)
          .await
        {
          Ok(_) => log::info!("HSM group removed successfully."),
          Err(e2) => log::debug!(
            "Error removing the HSM group. This always fails, ignore please. Reported: {e2}"
          ),
        }
      } else {
        log::debug!(
          "Parent HSM group {parent_hsm_group_name} is now empty and the option to delete empty groups has NOT been selected, will not remove it."
        );
      }
    }
  } else {
    log::info!("Dry run enabled, not modifying the HSM groups on the system.");
  }
  // *********************************************************************************************************
  // RETURN VALUES

  // *********************************************************************************************************
  // PRINT SOLUTIONS

  // Print target HSM data
  log::info!(
    "HSM '{target_hsm_group_name}' hw component summary: {target_hsm_hw_component_summary_hashmap:?}"
  );

  let target_hsm_group_value = serde_json::json!({
      "label": target_hsm_group_name,
      "decription": "",
      "members": target_hsm_node_vec,
      "tags": []
  });

  log::info!(
    "{}",
    serde_json::to_string_pretty(&target_hsm_group_value)
      .expect("infallible: json!{} -> string")
  );

  // Print parent HSM data
  log::info!(
    "HSM '{parent_hsm_group_name}' hw component summary: {parent_hsm_hw_component_summary_hashmap:?}"
  );

  let parent_hsm_group_value = serde_json::json!({
      "label": parent_hsm_group_name,
      "decription": "",
      "members": parent_hsm_node_vec,
      "tags": []
  });

  log::info!(
    "{}",
    serde_json::to_string_pretty(&parent_hsm_group_value)
      .expect("infallible: json!{} -> string")
  );

  Ok(())
}
