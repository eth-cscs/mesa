//! Helpers built on top of `ShastaClient::ims_image_*` methods.

use crate::{
  bos, common,
  error::Error,
  hsm::group::utils::get_member_vec_from_hsm_name_vec,
  ims::{self, image::http_client::types::Image},
};

/// Fuzzy lookup: return every image whose name *contains*
/// `image_name_opt`, restricted to the caller's available HSM groups.
///
/// Used to find images created by a CFS session that manta deliberately
/// leaves un-renamed (so the CFS session retains its original image ID).
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_fuzzy(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_name_available_vec: &[String],
  image_name_opt: Option<&str>,
  limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
  let mut image_available_vec: Vec<Image> = get_image_available_vec(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    hsm_name_available_vec,
    None, // NOTE: don't put any limit here since we may be looking in a large number of
          // HSM groups and we will filter the results by image name below
  )
  .await?;

  if let Some(image_name) = image_name_opt {
    image_available_vec.retain(|image| image.name.contains(image_name));
  }

  if let Some(limit_number) = limit_number_opt {
    // Limiting the number of results to return to client
    image_available_vec = image_available_vec[image_available_vec
      .len()
      .saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  Ok(image_available_vec.clone())
}

/// Return images whose name *exactly equals* `image_name`, restricted
/// to the caller's available HSM groups.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_by_name(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_name_available_vec: &[String],
  image_name: &str,
  limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
  let mut image_available_vec: Vec<Image> = get_image_available_vec(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    hsm_name_available_vec,
    None, // NOTE: don't put any limit here since we may be looking in a large number of
          // HSM groups and we will filter the results by image name below
  )
  .await?;

  image_available_vec.retain(|image| image.name.eq(image_name));

  if let Some(limit_number) = limit_number_opt {
    // Limiting the number of results to return to client
    image_available_vec = image_available_vec[image_available_vec
      .len()
      .saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  Ok(image_available_vec.clone())
}

/// Get Image using exact name match among the images available to the user based on the HSM groups
/// the user has access to. If no image is found with the exact name match, then, an error will be
/// returned.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn try_get_by_name(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  image_name: &str,
  limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
  // Get images available to the user
  // let mut image_available_vec: Vec<Image> = get_image_available_vec(
  //   shasta_token,
  //   shasta_base_url,
  //   shasta_root_cert,
  //   hsm_name_available_vec,
  //   None, // NOTE: don't put any limit here since we may be looking in a large number of
  //         // HSM groups and we will filter the results by image name below
  // )
  // .await?;

  let mut image_vec: Vec<Image> = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .ims_image_get_all(shasta_token)
  .await?;

  image_vec.retain(|image| image.name.eq(image_name));

  // If image name is provided, we try to find an image with the exact name match
  if image_vec.is_empty() {
    return Err(Error::ImageNotFound(image_name.to_string()));
  }

  if let Some(limit_number) = limit_number_opt {
    // Limiting the number of results to return to client
    image_vec = image_vec
      [image_vec.len().saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  Ok(image_vec.clone())
}

/// Just sorts images by creation time in ascendent order. Images with no
/// `created` timestamp sort before any with one (treated as empty string).
pub fn filter(image_vec: &mut [Image]) {
  image_vec.sort_by(|a, b| {
    a.created
      .as_deref()
      .unwrap_or("")
      .cmp(b.created.as_deref().unwrap_or(""))
  });
}

/// Fetch IMS images plus the CFS configurations, BOS session
/// templates, and boot-status they relate to, filtered by HSM group.
///
/// Returns a list of tuples
/// `(Image, cfs_configuration_name, targets, is_boot_image)` where
/// `targets` is either a list of HSM group names or xnames, and
/// `is_boot_image` indicates whether the image is currently used to
/// boot a node.
///
/// Filtering is performed against multiple sources to avoid missing
/// images: CFS sessions, BOS session templates, BSS boot parameters,
/// and a name-substring fallback. The CSM lookup is done once and the
/// `&ShastaClient`'s connection pool is reused for every
/// downstream call.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_with_details(
  client: &crate::ShastaClient,
  shasta_token: &str,
  hsm_group_name_vec: &[String],
  id_opt: Option<&str>,
  limit_number: Option<&u8>,
) -> Result<Vec<(Image, String, String, bool)>, Error> {
  let mut image_vec: Vec<Image> =
    client.ims_image_get(shasta_token, id_opt).await?;

  get_image_cfs_config_name_hsm_group_name(
    shasta_token,
    client.base_url(),
    client.root_cert(),
    &mut image_vec,
    hsm_group_name_vec,
    limit_number,
  )
  .await
  .map_err(|e| {
    Error::Message(format!("ERROR - Failed to get image details: {e}"))
  })
}

/// Resolve each IMS image to its CFS configuration, the HSM groups (or
/// xnames) it targets, and whether it is currently a boot image.
///
/// Returns a list of `(Image, cfs_configuration_name, targets,
/// is_boot_image)`. See [`get_with_details`] for the high-level
/// description of the matching strategy.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_image_cfs_config_name_hsm_group_name(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  image_vec: &mut Vec<Image>,
  hsm_group_name_vec: &[String],
  limit_number_opt: Option<&u8>,
) -> Result<Vec<(Image, String, String, bool)>, Error> {
  if let Some(limit_number) = limit_number_opt {
    // Limiting the number of results to return to client
    *image_vec = image_vec
      [image_vec.len().saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  let xname_vec = crate::hsm::group::utils::get_member_vec_from_hsm_name_vec(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    hsm_group_name_vec,
  )
  .await?;

  // Sort images by creation time order ASC
  // We need BOS session templates to find an image created by SAT
  let mut bos_sessiontemplate_value_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .bos_template_v2_get(shasta_token, None)
  .await?;

  let _ = bos::template::utils::filter(
    &mut bos_sessiontemplate_value_vec,
    None,
    hsm_group_name_vec,
    &xname_vec,
    None,
  );

  // We need CFS sessions to find images without a BOS session template (hopefully the CFS
  // session has not been deleted by CSCS staff, otherwise it will be technically impossible to
  // find unless we search images by HSM name and expect HSM name to be in image name...)
  let mut cfs_session_vec = crate::cfs::session::get_and_sort(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    None,
    None,
    None,
    None,
    Some(true),
  )
  .await?;

  crate::cfs::session::utils::filter(
    &mut cfs_session_vec,
    None,
    hsm_group_name_vec,
    &xname_vec,
    None,
    None,
    common::jwt_ops::is_user_admin(shasta_token),
  )?;

  let mut image_id_cfs_configuration_from_cfs_session: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            &cfs_session_vec,
        )?;

  image_id_cfs_configuration_from_cfs_session
    .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

  // Get IMAGES in nodes boot params. This is because CSCS staff deletes the CFS sessions and/or
  // BOS sessiontemplate breaking the history with actual state, therefore I need to go to boot
  // params to get the image id used to boot the nodes belonging to a HSM group
  let hsm_member_vec = get_member_vec_from_hsm_name_vec(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    hsm_group_name_vec,
  )
  .await?;

  let boot_param_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .bss_bootparameters_get_multiple(shasta_token, &hsm_member_vec)
  .await
  .unwrap_or_default();

  let image_id_from_boot_params: Vec<String> = boot_param_vec
    .iter()
    .map(crate::bss::types::BootParameters::get_boot_image)
    .collect();

  // Get Image details from IMS images API endpoint
  let mut image_detail_vec: Vec<(Image, String, String, bool)> = Vec::new();

  for image in image_vec {
    let image_id = image.id.as_ref().unwrap();

    let target_group_name_vec: Vec<String>;
    let cfs_configuration: String;
    let target_groups: String;

    if let Some(tuple) = image_id_cfs_configuration_from_cfs_session
      .iter()
      .find(|tuple| tuple.0.eq(image_id))
    {
      // Image details in CFS session
      cfs_configuration = tuple.clone().1;
      target_group_name_vec = tuple.2.clone();
      target_groups = target_group_name_vec.join(", ");
    } else if let Some(boot_params) = boot_param_vec
      .iter()
      .find(|boot_params| boot_params.get_boot_image().eq(image_id))
    {
      // Image details where image is found in a node boot param related to HSM we are
      // working with
      // Boot params don't have CFS configuration information
      cfs_configuration = "Not found".to_string();
      target_groups = boot_params.hosts.clone().join(",");
    } else if hsm_group_name_vec
      .iter()
      .any(|hsm_group_name| image.name.contains(hsm_group_name))
    {
      // Image details where image name contains HSM group name available to the user.
      // Boot params don't have CFS configuration information
      // NOTE: CSCS specific
      cfs_configuration = "Not found".to_string();

      target_groups = "Not found".to_string();
    } else {
      continue;
    }

    // NOTE: 'boot_image' needs to be processed outside the 'if' statement. Otherwise we may
    // miss images used to boot nodes filtered by a different branch in the 'if' statement
    let boot_image: bool = image_id_from_boot_params.contains(image_id);

    image_detail_vec.push((
      image.clone(),
      cfs_configuration.clone(),
      target_groups.clone(),
      boot_image,
    ));
  }

  Ok(image_detail_vec)
}

/// Returns a list of images available to the user based on the HSM groups the user has access to.
/// The method defines the images available to the user based on the following rules:
///  - If image is related to a BOS sessiontemplate related to a HSM group the user has access to, then, the image will be available to the user
///  - If image was created using a CFS session with HSM groups related to the user, then the image will be available to the user
///  - If image name contains HSM group the user is working on, then, the image will be available
///  to the user (NOTE: this is a bad practice because this is a free text prone to human errors
///  but we are extending the rules that defines if a user has access to an image because CSCS
///  staff deletes CFS sessions and BOS sessiontemplates so we may miss images related to the user
///  if we don't extend the rules)
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_image_available_vec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_name_available_vec: &[String],
  limit_number_opt: Option<&u8>,
) -> Result<Vec<Image>, Error> {
  let mut image_vec: Vec<Image> = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .ims_image_get_all(shasta_token)
  .await?;

  ims::image::utils::filter(&mut image_vec);

  // We need BOS session templates to find an image created by SAT
  let mut bos_sessiontemplate_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .bos_template_v2_get(shasta_token, None)
  .await?;

  let xname_from_group_vec =
    crate::hsm::group::utils::get_member_vec_from_hsm_name_vec(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      hsm_name_available_vec,
    )
    .await?;

  // Filter BOS sessiontemplates to the ones the user has access to
  let _ = bos::template::utils::filter(
    &mut bos_sessiontemplate_vec,
    None,
    hsm_name_available_vec,
    &xname_from_group_vec,
    None,
  );

  // We need CFS sessions to find images without a BOS session template
  let mut cfs_session_vec = crate::cfs::session::get_and_sort(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    None,
    None,
    None,
    None,
    Some(true),
  )
  .await?;

  // Filter CFS sessions to the ones the user has access to
  crate::cfs::session::utils::filter(
    &mut cfs_session_vec,
    None,
    hsm_name_available_vec,
    &xname_from_group_vec,
    None,
    None,
    true,
  )?;

  let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = crate::bos::template::utils::get_image_id_cfs_configuration_target_tuple_vec(
        &bos_sessiontemplate_vec,
    );

  image_id_cfs_configuration_from_bos_sessiontemplate
    .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

  let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        crate::cfs::session::utils::get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
            &cfs_session_vec,
        )?;

  image_id_cfs_configuration_from_cfs_session_vec
    .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

  let mut image_available_vec: Vec<Image> = Vec::new();

  for image in &image_vec {
    let image_id = image.id.as_ref().unwrap();

    if image_id_cfs_configuration_from_bos_sessiontemplate
      .iter()
      .any(|tuple| tuple.0.eq(image_id))
    {
      // If image is related to a BOS sessiontemplate related to a HSM group the user has
      // access to, then, we include this image to the list of images available to the user
      image_available_vec.push(image.clone());
    } else if image_id_cfs_configuration_from_cfs_session_vec
      .iter()
      .any(|tuple| tuple.0.eq(image_id))
    {
      // If image was created using a CFS session with HSM groups related to the user, then
      // we include this image to the list of images available to the user
      // FIXME: this needs to go away if we extend groups in CFS sessions to technology
      // rather than clusters
      image_available_vec.push(image.clone());
    } else if hsm_name_available_vec
      .iter()
      .any(|hsm_group_name| image.name.contains(hsm_group_name))
    {
      // If image name contains HSM group the user is working on, then, we include the image
      // to the list of images available to the user
      // FIXME: this should not be allowed... but CSCS staff deletes the CFS sessions so we
      // are extending the rules that defines if a user has access to an image
      image_available_vec.push(image.clone());
    } else if image.name.to_lowercase().contains("generic") {
      // If image is generic (meaning image name contains the word "generic"), then, the image
      // will be available to everyone, therefore it should be included to the list of images
      // available to the user
      // FIXME: This is should not be allowed since it is too vague, we concept of generic is
      // not limited to anything, a tenant may create an image which name contains "generic"
      // but they don't want to share it with other tenants meaning the scope of generic here
      // does not moves across tenants boundaries
      image_available_vec.push(image.clone());
    }

    // let target_groups = target_group_name_vec.join(", ");
  }

  if let Some(limit_number) = limit_number_opt {
    // Limiting the number of results to return to client
    image_available_vec = image_available_vec[image_available_vec
      .len()
      .saturating_sub(*limit_number as usize)..]
      .to_vec();
  }

  Ok(image_available_vec)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn image(name: &str, created: Option<&str>) -> Image {
    Image {
      id: Some(format!("id-{name}")),
      created: created.map(str::to_string),
      name: name.to_string(),
      link: None,
      arch: None,
      metadata: None,
    }
  }

  // ---------- filter (sorts by created ASC) ----------

  #[test]
  fn filter_sorts_by_created_ascending() {
    let mut images = vec![
      image("c", Some("2024-03-01T00:00:00Z")),
      image("a", Some("2024-01-01T00:00:00Z")),
      image("b", Some("2024-02-01T00:00:00Z")),
    ];
    filter(&mut images);
    let names: Vec<&str> = images.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
  }

  #[test]
  fn filter_is_stable_for_equal_timestamps() {
    let mut images = vec![
      image("first", Some("2024-01-01T00:00:00Z")),
      image("second", Some("2024-01-01T00:00:00Z")),
      image("third", Some("2024-01-01T00:00:00Z")),
    ];
    filter(&mut images);
    // Rust's sort_by is stable, so equal keys preserve insertion order.
    let names: Vec<&str> = images.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["first", "second", "third"]);
  }

  #[test]
  fn filter_empty_input_does_not_panic() {
    let mut images: Vec<Image> = vec![];
    filter(&mut images);
    assert!(images.is_empty());
  }

  #[test]
  fn filter_single_element_is_idempotent() {
    let mut images = vec![image("only", Some("2024-01-01T00:00:00Z"))];
    filter(&mut images);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].name, "only");
  }

  #[test]
  fn filter_treats_missing_created_as_empty_and_sorts_first() {
    let mut images = vec![
      image("b", Some("2024-01-01T00:00:00Z")),
      image("missing", None),
      image("a", Some("2024-02-01T00:00:00Z")),
    ];
    filter(&mut images);
    let names: Vec<&str> = images.iter().map(|i| i.name.as_str()).collect();
    // "" < any non-empty timestamp, so the missing-created image sorts first.
    assert_eq!(names, vec!["missing", "b", "a"]);
  }
}
