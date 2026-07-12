//! Building blocks for [`super::command::exec`] (component counting, pattern matching).

use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::{error::Error, hsm};
use serde_json::Value;
use tokio::sync::Semaphore;

/// `(node_xname, hw_component_name -> count)` for one node.
pub type NodeHwComponentCount = (String, HashMap<String, usize>);

/// Solve the hardware-pinning problem: pick xnames from the parent HSM
/// to satisfy `user_defined_target_hsm_hw_component_count_hashmap`,
/// returning `(target_hsm, parent_hsm)` — the left element is the
/// nodes moved into the target group, the right is what remains in the
/// parent.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub fn resolve_hw_description_to_xnames(
  mut target_hsm_node_hw_component_count_vec: Vec<NodeHwComponentCount>,
  mut parent_hsm_node_hw_component_count_vec: Vec<NodeHwComponentCount>,
  user_defined_target_hsm_hw_component_count_hashmap: HashMap<String, usize>,
) -> Result<(Vec<NodeHwComponentCount>, Vec<NodeHwComponentCount>), Error> {
  // *********************************************************************************************************
  // CALCULATE 'COMBINED HSM' WITH TARGET HSM AND PARENT HSM ELEMENTS COMBINED
  // NOTE: PARENT HSM may contain elements in TARGET HSM, we need to only add those xnames
  // which are not part of PARENT HSM already

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

  // *********************************************************************************************************
  // CALCULATE HW COMPONENT TYPE SCORE BASED ON SCARCITY

  // Get parent HSM group members
  // Calculate nomarlized score for each hw component type in as much HSM groups as possible
  // related to the stakeholders using these nodes
  let hw_component_scarcity_scores_hashmap: HashMap<String, f32> =
    calculate_hw_component_scarcity_scores(
      &combined_target_parent_hsm_node_hw_component_count_vec,
    );

  // *********************************************************************************************************
  // CALCULATE FINAL HSM SUMMARY COUNTERS AFTER REMOVING THE NODES THAT NEED TO GO TO TARGET
  // HSM (SUBSTRACT USER INPUT SUMMARY FROM INITIAL COMBINED HSM SUMMARY)
  let mut final_combined_target_parent_hsm_hw_component_summary =
    user_defined_target_hsm_hw_component_count_hashmap.clone();

  for (hw_component, qty) in
    combined_target_parent_hsm_hw_component_summary_hashmap
  {
    final_combined_target_parent_hsm_hw_component_summary
      .entry(hw_component)
      .and_modify(|current_qty| *current_qty = qty - *current_qty);
  }

  // Calculate new target HSM group
  let hw_component_counters_to_move_out_from_combined_hsm =
    calculate_target_hsm_pin(
      &final_combined_target_parent_hsm_hw_component_summary.clone(),
      &mut combined_target_parent_hsm_node_hw_component_count_vec,
      &mut target_hsm_node_hw_component_count_vec,
      &mut parent_hsm_node_hw_component_count_vec,
      &hw_component_scarcity_scores_hashmap,
    )?;

  let new_target_hsm_node_hw_component_count_vec =
    hw_component_counters_to_move_out_from_combined_hsm;

  Ok((
    new_target_hsm_node_hw_component_count_vec,
    combined_target_parent_hsm_node_hw_component_count_vec,
  ))
}

/// Pin means this function should be used when the user wants to keep as much nodes in
/// original target HSM group as possible. Use case for this:
///  - cluster upscaling or downscaling in same site and want to minimize the impact on running
///  applications
///  - defining final state for a cluster
pub fn get_best_candidate_in_hsm_pin(
  hsm_score_vec: &mut [(String, f32)],
  hsm_hw_component_vec: &[(String, HashMap<String, usize>)],
) -> Option<((String, f32), HashMap<String, usize>)> {
  if hsm_score_vec.is_empty() || hsm_hw_component_vec.is_empty() {
    return None;
  }

  hsm_score_vec.sort_by_key(|elem| elem.0.clone());
  // f32 partial_cmp returns None for NaN; treat NaN as equal so the sort stays
  // total-ordered without panicking on degenerate input.
  hsm_score_vec
    .sort_by(|b, a| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

  // Get node with highest normalized score (best candidate).
  // Non-empty check above guarantees first() is Some.
  let best_candidate: (String, f32) = hsm_score_vec
    .first()
    .expect("non-empty: checked above")
    .clone();

  hsm_hw_component_vec
    .iter()
    .find(|(node, _)| node.eq(&best_candidate.0))
    .map(|best_candiate| (best_candidate, best_candiate.1.clone()))
}

/// Pick the highest-scoring candidate node across both target and
/// parent HSM scoring vectors; updates the target vector in place if
/// the winner came from the target side.
pub fn get_best_candidate_in_target_and_parent_hsm_pin(
  target_hsm_node_score_tuple_vec: &mut [(String, f32)],
  parent_hsm_node_score_tuple_vec: &mut [(String, f32)],
  target_hsm_node_hw_component_count_vec: &mut [(
    String,
    HashMap<String, usize>,
  )],
  parent_hsm_node_hw_component_count_vec: &[(String, HashMap<String, usize>)],
) -> Option<((String, f32), HashMap<String, usize>)> {
  // Get best candidate in 'target' HSM group
  let target_best_candidate_tuple = get_best_candidate_in_hsm_pin(
    target_hsm_node_score_tuple_vec,
    target_hsm_node_hw_component_count_vec,
  );

  // Get best candidate in 'parent' HSM group
  let parent_best_candidate_tuple = get_best_candidate_in_hsm_pin(
    parent_hsm_node_score_tuple_vec,
    parent_hsm_node_hw_component_count_vec,
  );

  // If best candidate exists (in 'target' HSM group), then use it. Otherwise, use the one in 'parent' HSM group
  if target_best_candidate_tuple.is_some() {
    target_best_candidate_tuple
  } else if parent_best_candidate_tuple.is_some() {
    parent_best_candidate_tuple
  } else {
    None
  }
}

/// Generates a list of tuples with xnames and the hardware summary for each node. This method
/// keeps as much nodes from the target HSM group as it can, this is good to minimize the
/// number of nodes being changed in the cluster
/// Returns a list of tuples, the first element is the xname and the last element is a hardware
/// summary of the node
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub fn calculate_target_hsm_pin(
  user_defined_hsm_hw_components_count_hashmap: &HashMap<String, usize>, // hw
  // components summary the target hsm group should have according to user requests (this is
  // equivalent to target_hsm_node_hw_component_count_vec minus
  // hw_components_deltas_from_target_hsm_to_parent_hsm). Note hw componets needs to be grouped/filtered
  // based on user input
  combination_target_parent_hsm_node_hw_component_count_vec: &mut Vec<
    NodeHwComponentCount,
  >, // list
  // of hw component counters in target HSM group
  target_hsm_node_hw_component_count_vec: &mut Vec<NodeHwComponentCount>,
  parent_hsm_node_hw_component_count_vec: &mut Vec<NodeHwComponentCount>,
  hw_component_scarcity_scores_hashmap: &HashMap<String, f32>, // hw
                                                               // component type score for as much hsm groups related to the stakeholders using these
                                                               // nodes
) -> Result<Vec<NodeHwComponentCount>, Error> {
  ////////////////////////////////
  // Initialize

  // Calculate hw component counters for the whole HSM group
  let mut combination_target_parent_hsm_hw_component_summary_hashmap: HashMap<
    String,
    usize,
  > = calculate_hsm_hw_component_summary(
    combination_target_parent_hsm_node_hw_component_count_vec,
  );
  // Calculate hw component counters for the whole HSM group
  let target_hsm_hw_component_summary_hashmap: HashMap<String, usize> =
    calculate_hsm_hw_component_summary(target_hsm_node_hw_component_count_vec);
  // Calculate hw component counters for the whole HSM group
  let parent_hsm_hw_component_summary_hashmap: HashMap<String, usize> =
    calculate_hsm_hw_component_summary(parent_hsm_node_hw_component_count_vec);

  // Calculate initial scores for 'target' HSM group
  let mut target_hsm_node_score_tuple_vec: Vec<(String, f32)> =
    calculate_hsm_node_scores_from_final_hsm(
      target_hsm_node_hw_component_count_vec,
      &target_hsm_hw_component_summary_hashmap,
      user_defined_hsm_hw_components_count_hashmap,
      hw_component_scarcity_scores_hashmap,
    );

  // Calculate initial scores for 'parent' HSM group
  let mut parent_hsm_node_score_tuple_vec: Vec<(String, f32)> =
    calculate_hsm_node_scores_from_final_hsm(
      parent_hsm_node_hw_component_count_vec,
      &parent_hsm_hw_component_summary_hashmap,
      user_defined_hsm_hw_components_count_hashmap,
      hw_component_scarcity_scores_hashmap,
    );

  // Calculate hashmap to group nodes by score for 'target' HSM group
  let mut group_target_hsm_node_by_score_hashmap: HashMap<usize, Vec<String>> =
    HashMap::new();
  for (node, score) in &target_hsm_node_score_tuple_vec {
    group_target_hsm_node_by_score_hashmap
      .entry(*score as usize)
      .and_modify(|node_vec| node_vec.push(node.clone()))
      .or_insert(vec![node.clone()]);
  }

  // Calculate hashmap to group nodes by score for 'parent' HSM group
  let mut group_parent_hsm_node_by_score_hashmap: HashMap<usize, Vec<String>> =
    HashMap::new();
  for (node, score) in &parent_hsm_node_score_tuple_vec {
    group_parent_hsm_node_by_score_hashmap
      .entry(*score as usize)
      .and_modify(|node_vec| node_vec.push(node.clone()))
      .or_insert(vec![node.clone()]);
  }

  let mut nodes_migrated_from_combination_target_parent_hsm: Vec<(
    String,
    HashMap<String, usize>,
  )> = Vec::new();

  let (mut best_candidate, mut best_candidate_counters) =
    get_best_candidate_in_target_and_parent_hsm_pin(
      &mut target_hsm_node_score_tuple_vec,
      &mut parent_hsm_node_score_tuple_vec,
      target_hsm_node_hw_component_count_vec,
      parent_hsm_node_hw_component_count_vec,
    )
    .ok_or_else(|| {
      Error::Message("no best candidate found.".to_string())
      /* log::warn!("ERROR - No best candidate found.");
      std::process::exit(1); */
    })?;

  // Check if we need to keep iterating
  let mut work_to_do = keep_iterating_final_hsm(
    user_defined_hsm_hw_components_count_hashmap,
    &combination_target_parent_hsm_hw_component_summary_hashmap,
  );

  ////////////////////////////////
  // Iterate

  let mut iter = 0;

  while work_to_do {
    log::debug!("----- ITERATION {iter} -----");

    log::debug!(
      "HSM group hw component counters: {combination_target_parent_hsm_hw_component_summary_hashmap:?}"
    );
    log::debug!(
      "Final hw component counters the user wants: {user_defined_hsm_hw_components_count_hashmap:?}"
    );
    log::debug!(
      "Best candidate is '{}' with score {} and hw component counters {:?}",
      best_candidate.0,
      best_candidate.1,
      best_candidate_counters
    );

    ////////////////////////////////
    // Apply changes - Migrate from target to parent HSM

    // Add best candidate to list of nodes migrated
    nodes_migrated_from_combination_target_parent_hsm
      .push((best_candidate.0.clone(), best_candidate_counters.clone()));

    // Remove best candidate from combined HSM group
    combination_target_parent_hsm_node_hw_component_count_vec
      .retain(|(node, _)| !node.eq(&best_candidate.0));

    // Remove best candidate from target HSM group
    target_hsm_node_hw_component_count_vec
      .retain(|(node, _)| !node.eq(&best_candidate.0));

    // Remove best candidate from parent HSM group
    parent_hsm_node_hw_component_count_vec
      .retain(|(node, _)| !node.eq(&best_candidate.0));

    if combination_target_parent_hsm_node_hw_component_count_vec.is_empty() {
      break;
    }

    // Calculate hw component couters for the whole HSM group
    combination_target_parent_hsm_hw_component_summary_hashmap =
      calculate_hsm_hw_component_summary(
        combination_target_parent_hsm_node_hw_component_count_vec,
      );

    // Remove best candidate in target HSM group scores
    target_hsm_node_score_tuple_vec
      .retain(|(node, _)| !node.eq(&best_candidate.0));

    // Remove best candidate in parent HSM group scores
    parent_hsm_node_score_tuple_vec
      .retain(|(node, _)| !node.eq(&best_candidate.0));

    // Recalculate scores for 'target' HSM group
    let mut target_hsm_node_score_tuple_vec: Vec<(String, f32)> =
      calculate_hsm_node_scores_from_final_hsm(
        target_hsm_node_hw_component_count_vec,
        &combination_target_parent_hsm_hw_component_summary_hashmap,
        user_defined_hsm_hw_components_count_hashmap,
        hw_component_scarcity_scores_hashmap,
      );

    // Recalculate scores for 'parent' HSM group
    let mut parent_hsm_node_score_tuple_vec: Vec<(String, f32)> =
      calculate_hsm_node_scores_from_final_hsm(
        parent_hsm_node_hw_component_count_vec,
        &combination_target_parent_hsm_hw_component_summary_hashmap,
        user_defined_hsm_hw_components_count_hashmap,
        hw_component_scarcity_scores_hashmap,
      );

    // Calculate hashmap to group nodes by score
    let mut group_target_hsm_node_by_score_hashmap: HashMap<
      usize,
      Vec<String>,
    > = HashMap::new();
    for (node, score) in &target_hsm_node_score_tuple_vec {
      group_target_hsm_node_by_score_hashmap
        .entry(*score as usize)
        .and_modify(|node_vec| node_vec.push(node.clone()))
        .or_insert(vec![node.clone()]);
    }

    // Calculate hashmap to group nodes by score
    let mut group_parent_hsm_node_by_score_hashmap: HashMap<
      usize,
      Vec<String>,
    > = HashMap::new();
    for (node, score) in &parent_hsm_node_score_tuple_vec {
      group_parent_hsm_node_by_score_hashmap
        .entry(*score as usize)
        .and_modify(|node_vec| node_vec.push(node.clone()))
        .or_insert(vec![node.clone()]);
    }

    // Get best candidate in 'target' HSM group
    (best_candidate, best_candidate_counters) =
      get_best_candidate_in_target_and_parent_hsm_pin(
        &mut target_hsm_node_score_tuple_vec,
        &mut parent_hsm_node_score_tuple_vec,
        target_hsm_node_hw_component_count_vec,
        parent_hsm_node_hw_component_count_vec,
      )
      .ok_or_else(|| {
        Error::Message("no best candidate found.".to_string())
      })?;

    // Check if we need to keep iterating
    work_to_do = keep_iterating_final_hsm(
      user_defined_hsm_hw_components_count_hashmap,
      &combination_target_parent_hsm_hw_component_summary_hashmap,
    );

    iter += 1;
  }

  log::debug!("----- FINAL RESULT -----");

  log::debug!("No candidates found");

  Ok(nodes_migrated_from_combination_target_parent_hsm)
}

/// Compute a scarcity score per hardware component name — components
/// that appear fewer times across the HSM score higher. Used to
/// prioritise nodes carrying rare hardware during pinning.
#[must_use]
pub fn calculate_hw_component_scarcity_scores(
  hsm_node_hw_component_count: &Vec<(String, HashMap<String, usize>)>,
) -> HashMap<String, f32> {
  let total_num_hw_components: usize = hsm_node_hw_component_count
    .iter()
    .flat_map(|(_, hw_component_qty_hashmap)| hw_component_qty_hashmap.values())
    .sum();

  let mut hw_component_vec: Vec<&String> = hsm_node_hw_component_count
    .iter()
    .flat_map(|(_, hw_component_counter_hashmap)| {
      hw_component_counter_hashmap.keys()
    })
    .collect();

  hw_component_vec.sort();
  hw_component_vec.dedup();

  let mut hw_component_scarcity_score_hashmap: HashMap<String, f32> =
    HashMap::new();
  for hw_component in hw_component_vec {
    let mut hsm_hw_component_count = 0;

    for (_, hw_component_counter_hashmap) in hsm_node_hw_component_count {
      if let Some(hw_component_qty) =
        hw_component_counter_hashmap.get(hw_component)
      {
        hsm_hw_component_count += hw_component_qty;
      }
    }

    hw_component_scarcity_score_hashmap.insert(
      hw_component.clone(),
      (total_num_hw_components as f32) / (hsm_hw_component_count as f32),
    );
  }

  log::debug!(
    "Hw component scarcity scores: {hw_component_scarcity_score_hashmap:?}"
  );

  hw_component_scarcity_score_hashmap
}

/// Calculates a normalized score for each hw component in HSM group based on component
/// scarcity.
#[must_use]
pub fn calculate_hsm_node_scores_from_final_hsm(
  parent_hsm_node_hw_component_count_vec: &Vec<(
    String,
    HashMap<String, usize>,
  )>,
  parent_hsm_hw_component_summary_hashmap: &HashMap<String, usize>,
  final_hsm_summary_hashmap: &HashMap<String, usize>,
  hw_component_scarcity_scores_hashmap: &HashMap<String, f32>,
) -> Vec<(String, f32)> {
  let mut node_score_vec: Vec<(String, f32)> = Vec::new();

  for (xname, hw_component_count) in parent_hsm_node_hw_component_count_vec {
    let mut node_score: f32 = 0.0;
    for (hw_component, qty) in hw_component_count {
      // Missing scarcity score → treat as 0.0 (no penalty/reward) rather than
      // panic. Missing summary entries are handled by the get() pattern.
      let scarcity = *hw_component_scarcity_scores_hashmap
        .get(hw_component)
        .unwrap_or(&0.0);
      match (
        final_hsm_summary_hashmap.get(hw_component),
        parent_hsm_hw_component_summary_hashmap.get(hw_component),
      ) {
        (None, _) => {
          // final/user request does NOT contain hw component → penalize
          node_score -= scarcity * *qty as f32;
        }
        (Some(final_count), Some(parent_count))
          if final_count < parent_count =>
        {
          // parent has more than user requested → reward removing this node
          node_score += scarcity * *qty as f32;
        }
        _ => {
          // parent has <= user requested → penalize removing this node
          node_score -= scarcity * *qty as f32;
        }
      }
    }
    node_score_vec.push((xname.clone(), node_score));
  }

  node_score_vec
}

/// Return `true` while the current HSM summary still exceeds the
/// target (final) per-component minimum — i.e. there's slack to keep
/// moving nodes during pinning.
#[must_use]
pub fn keep_iterating_final_hsm(
  hsm_final_hw_component_summary_hashmap: &HashMap<String, usize>, // hw components in
  // the target hsm group asked by the user (this is the minimum boundary, we can't provide
  // less than this)
  // best_candidate_counters: &HashMap<String, usize>,
  // hw_components_deltas_from_target_hsm_to_parent_hsm: &HashMap<String, isize>, // minimum boundaries (we
  // can't provide less that this)
  hsm_current_hw_component_summary_hashmap: &HashMap<String, usize>, // list of nodes
                                                                     // and its scores
) -> bool {
  for (hw_component, final_qty) in hsm_final_hw_component_summary_hashmap {
    if hsm_current_hw_component_summary_hashmap
      .get(hw_component)
      .is_some_and(|current_qty| current_qty > final_qty)
    {
      return true;
    }
  }

  false
}

/// Returns a triple like (`xname`, `list of hw components`, `list of memory capacity`).
/// Note: list of hw components can be either the hw componentn pattern provided by user or the
/// description from the HSM API
/// NOTE: backend it not borrowed because we need to clone it in order to use it across threads
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_node_hw_component_count(
  shasta_token: String,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_member: &str,
  user_defined_hw_profile_vec: Vec<String>,
) -> Result<(String, Vec<String>, Vec<u64>), Error> {
  let hw_inventory = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_hw_inventory_get_query(&shasta_token, hsm_member)
  .await?;

  // The downstream `get_node_hw_properties_from_value` and its three
  // `get_list_*_from_hw_inventory_value` helpers walk JSON paths
  // against a `serde_json::Value`. Now that the HTTP client returns
  // typed `HWInventory`, re-serialise here so those helpers keep
  // working unchanged. Future work could refactor the helpers to take
  // `&HWInventory` directly.
  let node_hw_inventory_value = serde_json::to_value(&hw_inventory)?;
  let node_hw_profile = get_node_hw_properties_from_value(
    &node_hw_inventory_value,
    user_defined_hw_profile_vec.clone(),
  );

  Ok((hsm_member.to_string(), node_hw_profile.0, node_hw_profile.1))
}

/// Sum per-node hardware component counts into a single
/// `component -> total` map for the HSM group.
#[must_use]
pub fn calculate_hsm_hw_component_summary(
  target_hsm_group_node_hw_component_vec: &Vec<(
    String,
    HashMap<String, usize>,
  )>,
) -> HashMap<String, usize> {
  let mut hsm_hw_component_count_hashmap = HashMap::new();

  for (_xname, node_hw_component_count_hashmap) in
    target_hsm_group_node_hw_component_vec
  {
    for (hw_component, &qty) in node_hw_component_count_hashmap {
      hsm_hw_component_count_hashmap
        .entry(hw_component.clone())
        .and_modify(|qty_aux| *qty_aux += qty)
        .or_insert(qty);
    }
  }

  hsm_hw_component_count_hashmap
}

/// Returns the properties in `hw_property_list` found in the `node_hw_inventory_value` which is
/// HSM hardware inventory API json response
#[must_use]
pub fn get_node_hw_properties_from_value(
  node_hw_inventory_value: &Value,
  hw_component_pattern_list: Vec<String>,
) -> (Vec<String>, Vec<u64>) {
  let processor_vec =
        hsm::hw_inventory::hw_component::utils::get_list_processor_model_from_hw_inventory_value(
            node_hw_inventory_value,
        )
        .unwrap_or_default();

  let accelerator_vec =
        hsm::hw_inventory::hw_component::utils::get_list_accelerator_model_from_hw_inventory_value(
            node_hw_inventory_value,
        )
        .unwrap_or_default();

  let processor_and_accelerator = [processor_vec, accelerator_vec].concat();

  let processor_and_accelerator_lowercase = processor_and_accelerator
    .iter()
    .map(|hw_component| hw_component.to_lowercase());

  let mut node_hw_component_pattern_vec = Vec::new();

  for actual_hw_component_pattern in processor_and_accelerator_lowercase {
    if let Some(hw_component_pattern) = hw_component_pattern_list
      .iter()
      .find(|&hw_component| actual_hw_component_pattern.contains(hw_component))
    {
      node_hw_component_pattern_vec.push(hw_component_pattern.clone());
    } else {
      node_hw_component_pattern_vec.push(actual_hw_component_pattern);
    }
  }

  let memory_vec =
        hsm::hw_inventory::hw_component::utils::get_list_memory_capacity_from_hw_inventory_value(
            node_hw_inventory_value,
        )
        .unwrap_or_default();

  (node_hw_component_pattern_vec, memory_vec)
}

/// For each member of `hsm_group_member_vec`, query HSM Hardware
/// Inventory and produce a `(xname, component -> count)` row covering
/// the `user_defined_hw_component_vec` of interest, with memory
/// normalised against the supplied LCM.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_hsm_node_hw_component_counter(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  user_defined_hw_component_vec: &[String],
  hsm_group_member_vec: &[String],
  mem_lcm: u64,
) -> Result<Vec<(String, HashMap<String, usize>)>, Error> {
  // Get HSM group members hw configurfation based on user input

  let start = Instant::now();

  let mut tasks = tokio::task::JoinSet::new();

  let sem = Arc::new(Semaphore::new(5)); // CSM 1.3.1 higher

  // Calculate HSM group hw component counters
  // List of node hw component counters belonging to target hsm group
  let mut target_hsm_node_hw_component_count_vec = Vec::new();

  // Get HW inventory details for parent HSM group
  #[allow(clippy::unnecessary_to_owned)]
  // `hsm_member` is moved into the `async move` block below
  for hsm_member in hsm_group_member_vec.iter().cloned() {
    let shasta_token_string = shasta_token.to_string();
    let shasta_base_url_string = shasta_base_url.to_string();
    let shasta_root_cert_vec = shasta_root_cert.to_vec();
    let user_defined_hw_component_vec =
      user_defined_hw_component_vec.to_owned();

    let permit = Arc::clone(&sem).acquire_owned().await;

    // log::debug!("user_defined_hw_profile_vec_aux: {:?}", user_defined_hw_profile_vec_aux);
    tasks.spawn(async move {
      let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

      get_node_hw_component_count(
        shasta_token_string,
        &shasta_base_url_string,
        &shasta_root_cert_vec,
        &hsm_member,
        user_defined_hw_component_vec,
      )
      .await
    });
  }

  while let Some(message) = tasks.join_next().await {
    let mut node_hw_component_vec_tuple = message??;

    node_hw_component_vec_tuple.1.sort();

    let mut node_hw_component_count_hashmap: HashMap<String, usize> =
      HashMap::new();

    for node_hw_property_vec in node_hw_component_vec_tuple.1 {
      let count = node_hw_component_count_hashmap
        .entry(node_hw_property_vec)
        .or_insert(0);
      *count += 1;
    }

    let node_memory_total_capacity: u64 =
      node_hw_component_vec_tuple.2.iter().sum();

    node_hw_component_count_hashmap.insert(
      "memory".to_string(),
      (node_memory_total_capacity / mem_lcm)
        .try_into()
        .unwrap_or(0),
    );

    target_hsm_node_hw_component_count_vec.push((
      node_hw_component_vec_tuple.0,
      node_hw_component_count_hashmap,
    ));
  }

  let duration = start.elapsed();
  log::debug!("Time elapsed to calculate hw components is: {duration:?}");

  Ok(target_hsm_node_hw_component_count_vec)
}
