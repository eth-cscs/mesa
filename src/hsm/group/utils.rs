//! Helpers built on top of `ShastaClient::hsm_group_*` methods.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::{
  error::Error,
  hsm::{
    self,
    group::{GroupExt, types::Group},
  },
  node::utils::validate_xnames_format_and_membership_against_single_hsm,
};

use super::types::Member;

/// Return the full HSM groups visible to the caller — all groups for
/// admins (`pa_admin` realm role), otherwise filtered to those named in
/// the caller's Keycloak roles, with site-wide groups stripped.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_group_available(
  shasta_auth_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
) -> Result<Vec<Group>, Error> {
  let mut group_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get_all(shasta_auth_token)
  .await
  .map_err(|e| Error::Message(e.to_string()))?;

  // Get HSM groups/Keycloak roles the user has access to from JWT token
  let realm_access_role_vec =
    crate::common::jwt_ops::get_roles(shasta_auth_token)?;

  if realm_access_role_vec.contains(&crate::hsm::group::hacks::PA_ADMIN.to_string()) {
    Ok(group_vec)
  } else {
    let available_groups_name = get_group_name_available(
      shasta_auth_token,
      shasta_base_url,
      shasta_root_cert,
    )
    .await?;

    // `group.label` is now `ResourceName(pub String)`; compare its inner
    // `String` against the `Vec<String>` of available group names.
    group_vec.retain(|group| available_groups_name.contains(&group.label.0));

    // Remove site-wide HSM groups (alps, prealps, …) — see
    // `hsm::group::hacks` module docs for why.
    let realm_access_role_filtered_vec =
      hsm::group::hacks::filter_system_hsm_groups(group_vec.clone());

    Ok(realm_access_role_filtered_vec)
  }
}

/// Return the names of HSM groups visible to the caller — all groups
/// for admins, otherwise derived from the JWT's Keycloak roles with
/// site-wide group names stripped.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_group_name_available(
  shasta_auth_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
) -> Result<Vec<String>, Error> {
  log::debug!("Get HSM names available from JWT or all");

  // Get HSM groups/Keycloak roles the user has access to from JWT token
  let realm_access_role_vec =
    crate::common::jwt_ops::get_roles(shasta_auth_token)?;

  if realm_access_role_vec.contains(&crate::hsm::group::hacks::PA_ADMIN.to_string()) {
    log::debug!("User is admin, getting all HSM groups in the system");
    let all_hsm_groups = crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .hsm_group_get_all(shasta_auth_token)
    .await?
    .iter()
    // Unwrap the `ResourceName` newtype to the underlying `String` so
    // the rest of this function — which builds `Vec<String>` for
    // downstream consumers — stays unchanged.
    .map(|hsm_value| hsm_value.label.0.clone())
    .collect::<Vec<String>>();

    // Remove site-wide HSM groups (alps, prealps, …) — see
    // `hsm::group::hacks` module docs for why.
    let mut all_hsm_groups_filtered =
      hsm::group::hacks::filter_system_hsm_group_names(all_hsm_groups.clone());

    all_hsm_groups_filtered.sort();

    Ok(all_hsm_groups_filtered)
  } else {
    log::debug!("User is not admin, getting HSM groups available from JWT");

    let realm_access_role_vec = hsm::group::hacks::filter_keycloak_roles(
      realm_access_role_vec
        .iter()
        .map(String::as_str)
        .collect::<Vec<&str>>()
        .as_slice(),
    );

    // Remove site-wide HSM groups (alps, prealps, …) — see
    // `hsm::group::hacks` module docs for why.
    let mut realm_access_role_filtered_vec =
      hsm::group::hacks::filter_system_hsm_group_names(
        realm_access_role_vec.clone(),
      );

    realm_access_role_filtered_vec.sort();

    Ok(realm_access_role_filtered_vec)
  }
}

/// Add a list of xnames to target HSM group
/// Returns the new list of nodes in target HSM group
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn add_member(
  auth_token: &str,
  base_url: &str,
  root_cert: &[u8],
  group_label: &str,
  new_member: &str,
) -> Result<Vec<String>, Error> {
  // Get HSM group from CSM
  let shasta_client = crate::ShastaClient::new(
    base_url,
    root_cert.to_vec(),
  )?;
  let group_vec = shasta_client
    .hsm_group_get(auth_token, Some(&[group_label.to_string()]), None)
    .await?;

  // Check if HSM group found
  if let Some(group) = group_vec.first().cloned().as_mut() {
    // Update HSM group with new memebers
    // Create Member struct
    let new_member = new_member.to_string();
    let member = crate::hsm::group::types::Member {
      id: Some(new_member.clone()),
    };

    // Update HSM group in CSM
    let _ = shasta_client
      .hsm_group_post_member(auth_token, group_label, member)
      .await?;

    // Push the new id into the in-memory members list. The earlier
    // shape (`group.get_members().push(new_member)`) was a bug —
    // `Group::get_members(&self) -> Vec<String>` returns by value, so
    // the push went to a throwaway. Mutate `members.ids` directly so
    // the post-call snapshot actually reflects the new member.
    //
    // Post-progenitor: `Members.ids` is `Vec<XNameRw100>` (not
    // `Option<Vec<String>>`), so wrap the raw xname in the newtype.
    // `Members100` is not `Default`, so build it explicitly with an
    // empty `ids` if absent.
    let members = group.members.get_or_insert_with(|| {
      crate::hsm::group::types::Members { ids: vec![] }
    });
    members
      .ids
      .push(crate::hsm::group::types::XNameRw100(new_member));

    Ok(group.get_members())
  } else {
    Err(Error::GroupNotFound(group_label.to_string()))
  }
}

/// Removes list of xnames from  HSM group
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn remove_hsm_members(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  target_hsm_group_name: &str,
  new_target_hsm_members: Vec<&str>,
  dryrun: bool,
) -> Result<Vec<String>, Error> {
  // Check nodes are valid xnames and they belong to parent HSM group
  if let Ok(false) = validate_xnames_format_and_membership_against_single_hsm(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    new_target_hsm_members.as_slice(),
    Some(target_hsm_group_name),
  )
  .await
  {
    let error_msg =
      format!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
    return Err(Error::Message(error_msg));
  }

  // get list of parent HSM group members
  let mut target_hsm_group_member_vec: Vec<String> =
    get_member_vec_from_hsm_group_name(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      target_hsm_group_name,
    )
    .await?;

  target_hsm_group_member_vec.retain(|parent_member| {
    !new_target_hsm_members.contains(&parent_member.as_str())
  });

  target_hsm_group_member_vec.sort();
  target_hsm_group_member_vec.dedup();

  // *********************************************************************************************************
  // UPDATE HSM GROUP MEMBERS IN CSM
  if dryrun {
    log::debug!(
      "Remove following nodes from HSM group {target_hsm_group_name}:\n{new_target_hsm_members:?}"
    );

    log::debug!("dry-run enabled, changes not persisted.");
  } else {
    let shasta_client = crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?;
    for xname in new_target_hsm_members {
      let _ = shasta_client
        .hsm_group_delete_member(shasta_token, target_hsm_group_name, xname)
        .await;
    }
  }

  Ok(target_hsm_group_member_vec)
}

/// Moves list of xnames from parent to target HSM group
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn migrate_hsm_members(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  target_hsm_group_name: &str,
  parent_hsm_group_name: &str,
  new_target_hsm_members: &[&str],
  dryrun: bool,
) -> Result<(Vec<String>, Vec<String>), Error> {
  // Check nodes are valid xnames and they belong to parent HSM group
  if let Ok(false) = validate_xnames_format_and_membership_against_single_hsm(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    new_target_hsm_members,
    Some(parent_hsm_group_name),
  )
  .await
  {
    let error_msg =
      format!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
    return Err(Error::Message(error_msg));
  }

  // get list of target HSM group members
  let mut target_hsm_group_member_vec: Vec<String> =
    get_member_vec_from_hsm_group_name(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      target_hsm_group_name,
    )
    .await?;

  // merge HSM group list with the list of xnames provided by the user
  target_hsm_group_member_vec
    .extend(new_target_hsm_members.iter().copied().map(str::to_string));

  target_hsm_group_member_vec.sort();
  target_hsm_group_member_vec.dedup();

  // get list of parent HSM group members
  let mut parent_hsm_group_member_vec: Vec<String> =
    get_member_vec_from_hsm_group_name(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      parent_hsm_group_name,
    )
    .await?;

  parent_hsm_group_member_vec.retain(|parent_member| {
    !target_hsm_group_member_vec.contains(parent_member)
  });

  parent_hsm_group_member_vec.sort();
  parent_hsm_group_member_vec.dedup();

  // *********************************************************************************************************
  // UPDATE HSM GROUP MEMBERS IN CSM
  if dryrun {
  } else {
    let shasta_client = crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?;
    for xname in new_target_hsm_members {
      let member = Member {
        id: Some(xname.to_string()),
      };

      let _ = shasta_client
        .hsm_group_post_member(shasta_token, target_hsm_group_name, member)
        .await;

      let _ = shasta_client
        .hsm_group_delete_member(shasta_token, parent_hsm_group_name, xname)
        .await;
    }
  }

  Ok((target_hsm_group_member_vec, parent_hsm_group_member_vec))
}

/// Receives 2 lists of xnames old xnames to remove from parent HSM group and new xhanges to add to target HSM group, and does just that
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn update_hsm_group_members(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_group_name: &str,
  old_target_hsm_group_members: &[&str],
  new_target_hsm_group_members: &[&str],
) -> Result<(), Error> {
  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  // Delete members
  for old_member in old_target_hsm_group_members {
    if !new_target_hsm_group_members.contains(old_member) {
      let _ = shasta_client
        .hsm_group_delete_member(shasta_token, hsm_group_name, old_member)
        .await;
    }
  }

  // Add members
  for new_member in new_target_hsm_group_members {
    if !old_target_hsm_group_members.contains(new_member) {
      let member = Member {
        id: Some(new_member.to_string()),
      };

      let _ = shasta_client
        .hsm_group_post_member(shasta_token, hsm_group_name, member)
        .await;
    }
  }

  Ok(())
}

/// Return a `HashMap` keyed by xname, valued with the group labels each
/// xname belongs to. Restricted to the provided `xname_vec`.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_xname_map_and_filter_by_xname_vec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  xname_vec: Vec<&str>,
) -> Result<HashMap<String, Vec<String>>, Error> {
  let hsm_group_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get_all(shasta_token)
  .await?;

  let mut xname_map: HashMap<String, Vec<String>> = HashMap::new();

  for hsm_group in hsm_group_vec {
    for xname in hsm_group.get_members() {
      if xname_vec.contains(&xname.as_str()) {
        xname_map
          .entry(xname)
          .and_modify(|group_vec| group_vec.push(hsm_group.label.0.clone()))
          .or_insert(vec![hsm_group.label.0.clone()]);
      }
    }
  }

  Ok(xname_map)
}

/// Return a `HashMap` keyed by HSM group label with the member xnames
/// as values, restricted to the given `hsm_name_vec` labels.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_hsm_map_and_filter_by_hsm_name_vec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_name_vec: &[&str],
) -> Result<HashMap<String, Vec<String>>, Error> {
  let hsm_group_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get_all(shasta_token)
  .await?;

  Ok(filter_by_hsm_group_name_and_convert_to_map(
    hsm_name_vec,
    &hsm_group_vec.iter().collect::<Vec<&Group>>(),
  ))
}

/// Return a `HashMap` keyed by HSM group label with member xnames as
/// values, restricted to groups containing any xname in `member_vec`.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_hsm_group_map_and_filter_by_hsm_group_member_vec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  member_vec: &[&str],
) -> Result<HashMap<String, Vec<String>>, Error> {
  let hsm_group_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get_all(shasta_token)
  .await?;

  Ok(filter_by_hsm_group_members_and_convert_to_map(
    member_vec,
    hsm_group_vec,
  ))
}

/// Given a list of `HsmGroup` struct and a list of Hsm group names, it will filter out those
/// not in the Hsm group names and convert from `HsmGroup` struct to `HashMap`
#[must_use]
pub fn filter_by_hsm_group_name_and_convert_to_map(
  hsm_name_vec: &[&str],
  hsm_group_vec: &[&Group],
) -> HashMap<String, Vec<String>> {
  let mut hsm_group_map: HashMap<String, Vec<String>> = HashMap::new();

  for hsm_group in hsm_group_vec {
    // `Group.label` is `ResourceName(pub String)`; reach into `.0` for
    // the `&str` comparison and for the map key.
    if hsm_name_vec.contains(&hsm_group.label.0.as_str()) {
      hsm_group_map.entry(hsm_group.label.0.clone()).or_insert(
        // `Members.ids` is now `Vec<XNameRw100>` (no `Option`). Map the
        // newtype out; an absent `members` block still yields `vec![]`
        // via the explicit `map_or_else` rather than a silent
        // `unwrap_or_default()` on the prior `Option<Vec<…>>`.
        hsm_group.members.as_ref().map_or_else(Vec::new, |members| {
          members.ids.iter().map(|x| x.0.clone()).collect()
        }),
      );
    }
  }

  hsm_group_map
}

/// Given a list of `HsmGroup` struct and a list of Hsm group members, it will filter out those
/// not in the Hsm group names and convert from `HsmGroup` struct to `HashMap`
#[must_use]
pub fn filter_by_hsm_group_members_and_convert_to_map(
  member_vec: &[&str],
  hsm_group_vec: Vec<Group>,
) -> HashMap<String, Vec<String>> {
  let mut hsm_group_map: HashMap<String, Vec<String>> = HashMap::new();

  for hsm_group in hsm_group_vec {
    if hsm_group
      .get_members()
      .iter()
      .any(|member| member_vec.contains(&member.as_str()))
    {
      // Same shape gymnastics as
      // `filter_by_hsm_group_name_and_convert_to_map`: unwrap the
      // `ResourceName` for the map key and translate `Vec<XNameRw100>`
      // to `Vec<String>` for the value.
      let key = hsm_group.label.0;
      hsm_group_map.entry(key).or_insert(
        hsm_group
          .members
          .map_or_else(Vec::new, |members| {
            members.ids.into_iter().map(|x| x.0).collect()
          }),
      );
    }
  }

  hsm_group_map
}

/// Extract `members.ids[]` xnames from an HSM group JSON Value.
pub fn get_member_vec_from_hsm_group_value(hsm_group: &Value) -> Vec<String> {
  // Take all nodes for all hsm_groups found and put them in a Vec
  hsm_group
    .pointer("/members/ids")
    .and_then(Value::as_array)
    .and_then(|member_vec| {
      member_vec
        .iter()
        .map(|xname| xname.as_str().map(str::to_string))
        .collect()
    })
    .unwrap_or_default()
}

/// Extract member xnames from a typed HSM `Group`.
#[must_use]
pub fn get_member_vec_from_hsm_group(hsm_group: &Group) -> Vec<String> {
  // Take all nodes for all hsm_groups found and put them in a Vec
  hsm_group.get_members()
}

/// Get the list of xnames which are members of a list of HSM groups.
///
/// Example: given HSM groups `tenant_a: [x1003c1s7b0n0, x1003c1s7b0n1]`
/// and `tenant_b: [x1003c1s7b1n0]`, calling with `hsm_name_vec:
/// &["tenant_a", "tenant_b"]` returns `[x1003c1s7b0n0, x1003c1s7b0n1,
/// x1003c1s7b1n0]`.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_member_vec_from_hsm_name_vec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_name_vec: &[String],
) -> Result<Vec<String>, Error> {
  log::debug!("Get xnames from HSM groups");
  log::debug!("Get xnames from HSM groups: {hsm_name_vec:?}");

  let hsm_group_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get(shasta_token, Some(hsm_name_vec), None)
  .await?;

  let mut hsm_group_member_vec: Vec<String> = Vec::new();

  for hsm_group in hsm_group_vec {
    hsm_group_member_vec.append(&mut hsm_group.get_members());
  }

  Ok(hsm_group_member_vec)
}

/// Collect the union of `members.ids[]` xnames across multiple HSM
/// group JSON Values, deduplicated into a `HashSet`.
pub fn get_member_vec_from_hsm_group_value_vec(
  hsm_groups: &[Value],
) -> HashSet<String> {
  hsm_groups
    .iter()
    .flat_map(get_member_vec_from_hsm_group_value)
    .collect()
}

/// Collect the union of member xnames across multiple typed HSM
/// `Group`s, deduplicated into a `HashSet`.
pub fn get_member_vec_from_hsm_group_vec(
  hsm_groups: &[Group],
) -> HashSet<String> {
  hsm_groups
    .iter()
    .flat_map(get_member_vec_from_hsm_group)
    .collect()
}

/// Returns a Map with nodes and the list of hsm groups that node belongs to.
/// eg "x1500b5c1n3 --> [ psi-dev, psi-dev_cn ]"
pub fn group_members_by_hsm_group_from_hsm_groups_value(
  hsm_groups: &Vec<Value>,
) -> HashMap<String, Vec<String>> {
  let mut member_hsm_map: HashMap<String, Vec<String>> = HashMap::new();
  for hsm_group_value in hsm_groups {
    let Some(hsm_group_name) = hsm_group_value
      .get("label")
      .and_then(Value::as_str)
      .map(str::to_string)
    else {
      log::warn!(
        "Skipping HSM group with missing or non-string 'label': {hsm_group_value}"
      );
      continue;
    };
    for member in get_member_vec_from_hsm_group_value(hsm_group_value) {
      member_hsm_map
        .entry(member)
        .and_modify(|hsm_groups| hsm_groups.push(hsm_group_name.clone()))
        .or_insert_with(|| vec![hsm_group_name.clone()]);
    }
  }

  member_hsm_map
}

/// Per-group member xnames for every HSM group whose label contains
/// `hsm_group_name_substring` (substring match).
///
/// Each entry in the returned vector corresponds to one matched group
/// and carries that group's member xnames; the outer length is the
/// number of groups matched (not the number of unique members).
#[derive(Debug)]
pub struct GroupMembers {
  /// xnames belonging to the matched HSM group.
  pub members: Vec<String>,
}

/// List the member xnames of every HSM group whose label contains
/// `hsm_group_name_substring` (substring match).
///
/// Lifted from the former `crate::common::cluster_ops::get_details`;
/// the operation is purely an HSM-group lookup + member expansion, so
/// it belongs in this namespace.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_members_for_groups_matching(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_group_name_substring: &str,
) -> Result<Vec<GroupMembers>, Error> {
  let hsm_group_value_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .hsm_group_get_hsm_group_vec(
    shasta_token,
    Some(&hsm_group_name_substring.to_string()),
  )
  .await?;

  Ok(
    hsm_group_value_vec
      .into_iter()
      .map(|hsm_group| GroupMembers {
        members: get_member_vec_from_hsm_group(&hsm_group),
      })
      .collect(),
  )
}

/// Fetch a single HSM group by label and return its member xnames.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_member_vec_from_hsm_group_name(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_group: &str,
) -> Result<Vec<String>, Error> {
  // Take all nodes for all hsm_groups found and put them in a Vec
  Ok(
    crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .hsm_group_get_one(shasta_token, hsm_group)
    .await?
    .get_members(),
  )
}
