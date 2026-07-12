use std::collections::{BTreeMap, HashMap};

use serde_yaml::Value;
use uuid::Uuid;

use crate::{
  bos::{BootSet, BosSession, BosSessionTemplate, Cfs, Operation},
  common::{self, yaml::yaml_str},
  error::Error,
  hsm,
  ims::{self, image::http_client::types::Link},
  node::utils::validate_target_hsm_members,
};

use super::{
  configuration, image,
  images::{
    filter_product_catalog_images, process_sat_file_image_ims_type_recipe,
    process_sat_file_image_old_version_struct,
    process_sat_file_image_product_type_ims_recipe,
  },
  sessiontemplate,
};

#[allow(clippy::too_many_arguments)]
/// Pre-flight validation for the SAT file's `session_templates`
/// section: rejects entries referencing missing images, unknown
/// configurations, or out-of-scope HSM groups / xnames.
pub async fn validate_sat_file_session_template_section(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  image_yaml_vec: &[image::Image],
  configuration_yaml_vec: &[configuration::Configuration],
  session_template_yaml_vec: &[sessiontemplate::SessionTemplate],
  hsm_group_available_vec: &[String],
) -> Result<(), Error> {
  // Validate 'session_template' section in SAT file
  log::debug!("Validate 'session_template' section in SAT file");
  for session_template_yaml in session_template_yaml_vec {
    // Validate session_template
    log::debug!(
      "Validate 'session_template' '{}'",
      session_template_yaml.name
    );

    // Validate user has access to HSM groups in 'session_template' section
    log::debug!(
      "Validate 'session_template' '{}' HSM groups",
      session_template_yaml.name
    );

    let bos_session_template_hsm_groups: Vec<String> =
      if let Some(boot_sets_compute) = session_template_yaml
        .bos_parameters
        .boot_sets
        .get("compute")
      {
        boot_sets_compute.node_groups.clone().unwrap_or_default()
      } else if let Some(boot_sets_uan) =
        session_template_yaml.bos_parameters.boot_sets.get("uan")
      {
        boot_sets_uan.node_groups.clone().unwrap_or_default()
      } else {
        return Err(Error::SatFile(
          "No HSM group found in session_templates section in SAT file"
            .to_string(),
        ));
      };

    for hsm_group in bos_session_template_hsm_groups {
      if !hsm_group_available_vec.contains(&hsm_group) {
        return Err(Error::SatFile(format!(
          "HSM group '{}' in session_templates {} not allowed, List of HSM groups available {:?}. Exit",
          hsm_group, session_template_yaml.name, hsm_group_available_vec
        )));
      }
    }

    // Validate boot image (session_template.image)
    log::debug!(
      "Validate 'session_template' '{}' boot image",
      session_template_yaml.name
    );

    if let sessiontemplate::Image::ImageRef {
      image_ref: ref_name_to_find,
    } = &session_template_yaml.image
    {
      // Validate image_ref (session_template.image.image_ref). Search in SAT file for any
      // image with images[].ref_name
      log::debug!("Searching ref_name '{ref_name_to_find}' in SAT file");

      let image_ref_name_found = image_yaml_vec
        .iter()
        .any(|image| image.ref_name.eq(&Some(ref_name_to_find).cloned()));

      if !image_ref_name_found {
        return Err(Error::SatFile(format!(
          "Could not find image ref '{ref_name_to_find}' in SAT file. Exit"
        )));
      }
    } else if let sessiontemplate::Image::Ims { ims } =
      &session_template_yaml.image
    {
      match ims {
        sessiontemplate::ImsDetails::Name {
          name: image_name_substr_to_find,
        } => {
          // Validate image name (session_template.image.ims.name). Search in SAT file and CSM
          log::debug!(
            "Searching image name '{}' related to session template '{}' in SAT file",
            image_name_substr_to_find,
            session_template_yaml.name
          );

          let mut image_found = image_yaml_vec
            .iter()
            .any(|image| image.name.eq(image_name_substr_to_find));

          if !image_found {
            log::warn!(
              "Image name '{image_name_substr_to_find}' not found in SAT file, looking in CSM"
            );
            log::debug!(
              "Searching image name '{}' related to session template '{}' in CSM",
              image_name_substr_to_find,
              session_template_yaml.name
            );

            image_found = ims::image::utils::try_get_by_name(
              shasta_token,
              shasta_base_url,
              shasta_root_cert,
              image_name_substr_to_find,
              Some(&1),
            )
            .await
            .is_ok();
          }

          if !image_found {
            return Err(Error::SatFile(format!(
              "Could not find image name '{}' in session_template '{}'. Exit",
              image_name_substr_to_find, session_template_yaml.name
            )));
          }
        }
        sessiontemplate::ImsDetails::Id { id: image_id } => {
          // Validate image id (session_template.image.ims.id) in CSM
          log::debug!(
            "Searching image id '{}' related to session template '{}' in CSM",
            image_id,
            session_template_yaml.name
          );

          let image_found = crate::ShastaClient::new(
            shasta_base_url,
            shasta_root_cert.to_vec(),
          )?
          .ims_image_get(shasta_token, Some(image_id.as_str()))
          .await
          .is_ok();

          if !image_found {
            return Err(Error::SatFile(format!(
              "Could not find image id '{}' in session_template '{}'. Exit",
              image_id, session_template_yaml.name
            )));
          }
        }
      }
    } else if let sessiontemplate::Image::ImageName(image_name) =
      &session_template_yaml.image
    {
      // Validate image name (session_template.image.image). Search in SAT file for any
      log::debug!("Searching image name '{image_name}' in SAT file");

      let image_ref_name_found = image_yaml_vec
        .iter()
        .any(|image| image.name.eq(&image_name.clone()));

      if !image_ref_name_found {
        return Err(Error::SatFile(format!(
          "Could not find image '{image_name}' in SAT file. Exit"
        )));
      }
    }

    // Validate configuration
    log::debug!(
      "Validate 'session_template' '{}' configuration",
      session_template_yaml.name
    );

    log::debug!(
      "Searching configuration name '{}' related to session template '{}' in CSM in SAT file",
      session_template_yaml.configuration,
      session_template_yaml.name
    );

    let mut configuration_found =
      configuration_yaml_vec.iter().any(|configuration_yaml| {
        configuration_yaml
          .name
          .eq(&session_template_yaml.configuration)
      });

    if !configuration_found {
      // CFS configuration in session_template not found in SAT file, searching in CSM
      log::warn!("Configuration not found in SAT file, looking in CSM");
      log::debug!(
        "Searching configuration name '{}' related to session_template '{}' in CSM",
        session_template_yaml.configuration,
        session_template_yaml.name
      );

      configuration_found = crate::ShastaClient::new(
        shasta_base_url,
        shasta_root_cert.to_vec(),
      )?
      .cfs_configuration_v3_get(
        shasta_token,
        Some(&session_template_yaml.configuration),
      )
      .await
      .is_ok();

      if !configuration_found {
        return Err(Error::SatFile(format!(
          "Could not find configuration '{}' in session_template '{}'. Exit",
          session_template_yaml.configuration, session_template_yaml.name,
        )));
      }
    }
  }

  Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Apply every entry in the SAT file's `session_templates` section:
/// rewrite image references using the freshly-built image IDs (from
/// `ref_name_processed_hashmap`) and PUT each template into BOS.
pub async fn process_session_template_section_in_sat_file(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  ref_name_processed_hashmap: HashMap<String, String>,
  hsm_group_available_vec: &[String],
  sat_file_yaml: Value,
  reboot: bool,
  dry_run: bool,
) -> Result<(Vec<BosSessionTemplate>, Vec<BosSession>), Error> {
  let empty_vec = Vec::new();
  let bos_session_template_list_yaml = sat_file_yaml
    .get("session_templates")
    .and_then(Value::as_sequence)
    .unwrap_or(&empty_vec);

  if bos_session_template_list_yaml.is_empty() {
    log::warn!(
      "No 'session_templates' section found in SAT file. Skipping session template processing"
    );
    return Ok((Vec::new(), Vec::new()));
  }

  let mut bos_st_created_vec: Vec<BosSessionTemplate> = Vec::new();
  let mut bos_sessions_created: Vec<BosSession> = Vec::new();

  for bos_sessiontemplate_yaml in bos_session_template_list_yaml {
    // Get boot image details in BOS sessiontemplate. This is needed to create the BOS
    // sessiontemplate BootSets
    let image_details: ims::image::http_client::types::Image =
      if let Some(bos_sessiontemplate_image) =
        bos_sessiontemplate_yaml.get("image")
      {
        let (image_reference, is_image_id) =
          get_image_reference_from_bos_sessiontemplate_yaml(
            bos_sessiontemplate_image,
            &ref_name_processed_hashmap,
          )?;
        if dry_run {
          let dry_run_mock_image =
            get_image_details_from_bos_sessiontemplate_yaml(
              shasta_token,
              shasta_base_url,
              shasta_root_cert,
              &image_reference,
              is_image_id,
            )
            .await
            .unwrap_or_else(|_| {
              // In dry run mode, generate a mock image

              if is_image_id {
                // Image reference is an image ID
                ims::image::http_client::types::Image {
                  id: Some(image_reference.clone()),
                  created: None,
                  name: "dryrun_image".to_string(),
                  link: Some(Link {
                    path: "dryrun_path".to_string(),
                    etag: Some("dryrun_etag".to_string()),
                    r#type: "dryrun_type".to_string(),
                  }),
                  arch: None,
                  metadata: None,
                }
              } else {
                // Image reference is an image name
                ims::image::http_client::types::Image {
                  id: None,
                  created: None,
                  name: image_reference.clone(),
                  link: Some(Link {
                    path: "dryrun_path".to_string(),
                    etag: Some("dryrun_etag".to_string()),
                    r#type: "dryrun_type".to_string(),
                  }),
                  arch: None,
                  metadata: None,
                }
              }
            });

          log::debug!(
            "Dry run mode: Generate mock Image\n{}",
            serde_json::to_string_pretty(&dry_run_mock_image)?
          );

          dry_run_mock_image
        } else {
          get_image_details_from_bos_sessiontemplate_yaml(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &image_reference,
            is_image_id,
          )
          .await?
        }
      } else {
        return Err(Error::SatFile(
          "ERROR: no 'image' section in session_template.\nExit".to_string(),
        ));
      };

    log::debug!("Image with name '{}' found", image_details.name);

    // Get CFS configuration to configure the nodes
    let bos_session_template_configuration_name =
      yaml_str(bos_sessiontemplate_yaml, "configuration")?.to_string();

    // Check CFS configuration exists in CSM
    log::debug!(
      "Looking for CFS configuration with name: {bos_session_template_configuration_name}"
    );

    if dry_run {
      log::debug!(
        "Dry run mode: CFS configuration '{bos_session_template_configuration_name}' found in CSM."
      );
    } else {
      crate::ShastaClient::new(
        shasta_base_url,
        shasta_root_cert.to_vec(),
      )?
      .cfs_configuration_v3_get(
        shasta_token,
        Some(&bos_session_template_configuration_name),
      )
      .await?;
    }

    // let ims_image_name = image_details.name.to_string();
    let image_link = image_details.link.as_ref().ok_or_else(|| {
      Error::SatFile(format!(
        "IMS image '{}' has no 'link' (no S3 manifest)",
        image_details.name
      ))
    })?;
    let ims_image_etag: &str = image_link.etag.as_deref().ok_or_else(|| {
      Error::SatFile(format!(
        "IMS image '{}' link has no 'etag'",
        image_details.name
      ))
    })?;
    let ims_image_path: &str = image_link.path.as_ref();
    let ims_image_type: &str = image_link.r#type.as_ref();

    let bos_sessiontemplate_name = bos_sessiontemplate_yaml
      .get("name")
      .and_then(Value::as_str)
      .map(str::to_string)
      .unwrap_or_default();

    let mut boot_set_vec: HashMap<String, BootSet> = HashMap::new();

    let boot_sets_mapping = bos_sessiontemplate_yaml
      .get("bos_parameters")
      .and_then(|bos_parameters| bos_parameters.get("boot_sets"))
      .and_then(Value::as_mapping)
      .ok_or_else(|| {
        Error::YamlShape(
          "SAT file: session_template is missing 'bos_parameters.boot_sets'"
            .to_string(),
        )
      })?;
    for (parameter, boot_set) in boot_sets_mapping {
      let kernel_parameters = boot_set
        .get("kernel_parameters")
        .and_then(Value::as_str)
        .ok_or_else(|| {
          Error::YamlShape(
            "SAT file: boot_set is missing 'kernel_parameters'".to_string(),
          )
        })?;
      let arch_opt = boot_set
        .get("arch")
        .and_then(Value::as_str)
        .map(str::to_string);

      let node_roles_groups_opt: Option<Vec<String>> = boot_set
        .get("node_roles_groups")
        .and_then(Value::as_sequence)
        .and_then(|node_role_groups| {
          node_role_groups
            .iter()
            .map(|hsm_group_value| hsm_group_value.as_str().map(str::to_string))
            .collect()
        });

      // Validate/check user can create BOS sessiontemplates based on node roles. Users
      // with tenant role are not allowed to create BOS sessiontemplates based on node roles
      // however admin tenants are allowed to create BOS sessiontemplates based on node roles
      if !hsm_group_available_vec.is_empty()
        && node_roles_groups_opt
          .clone()
          .is_some_and(|node_roles_groups| !node_roles_groups.is_empty())
      {
        return Err(Error::SatFile(
          "User type tenant can't user node roles in BOS sessiontemplate. Exit"
            .to_string(),
        ));
      }

      let node_groups_opt: Option<Vec<String>> = boot_set
        .get("node_groups")
        .and_then(Value::as_sequence)
        .and_then(|node_group| {
          node_group
            .iter()
            .map(|hsm_group_value| hsm_group_value.as_str().map(str::to_string))
            .collect()
        });

      // Strip site-wide group names — see `hsm::group::hacks` module
      // docs for why.
      let node_groups_opt = node_groups_opt.map(|node_groups| {
        hsm::group::hacks::filter_system_hsm_group_names(node_groups)
      });

      // Validate/check HSM groups in YAML file session_templates.bos_parameters.boot_sets.<parameter>.node_groups matches with
      // Check hsm groups in SAT file includes the hsm_group_param
      for node_group in node_groups_opt.clone().unwrap_or_default() {
        if !hsm_group_available_vec.contains(&node_group) {
          return Err(Error::SatFile(format!(
            "User does not have access to HSM group '{node_group}' in SAT file under session_templates.bos_parameters.boot_sets.compute.node_groups section. Exit"
          )));
        }
      }

      // Validate user has access to the xnames in the BOS sessiontemplate
      let node_list_opt: Option<Vec<String>> = boot_set
        .get("node_list")
        .and_then(Value::as_sequence)
        .and_then(|node_list| {
          node_list
            .iter()
            .map(|node_value_value| {
              node_value_value.as_str().map(str::to_string)
            })
            .collect()
        });

      // Validate user has access to the list of nodes in BOS sessiontemplate
      if let Some(node_list) = &node_list_opt {
        validate_target_hsm_members(
          shasta_token,
          shasta_base_url,
          shasta_root_cert,
          &node_list
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<&str>>(),
        )
        .await?;
      }

      let cfs = Cfs {
        configuration: Some(bos_session_template_configuration_name.clone()),
      };

      let rootfs_provider = boot_set
        .get("rootfs_provider")
        .and_then(Value::as_str)
        .map(str::to_string);
      let rootfs_provider_passthrough = boot_set
        .get("rootfs_provider_passthrough")
        .and_then(Value::as_str)
        .map(str::to_string);

      let boot_set = BootSet {
        name: None,
        path: Some(ims_image_path.to_string()),
        r#type: Some(ims_image_type.to_string()),
        etag: Some(ims_image_etag.to_string()),
        kernel_parameters: Some(kernel_parameters.to_string()),
        node_list: node_list_opt,
        node_roles_groups: node_roles_groups_opt,
        node_groups: node_groups_opt,
        rootfs_provider,
        rootfs_provider_passthrough,
        cfs: Some(cfs),
        arch: arch_opt,
      };

      let parameter_str = parameter.as_str().ok_or_else(|| {
        Error::YamlShape("SAT file: boot_set key is not a string".to_string())
      })?;
      boot_set_vec.insert(parameter_str.to_string(), boot_set);
    }

    let cfs = Cfs {
      configuration: Some(bos_session_template_configuration_name),
    };

    let create_bos_session_template_payload = BosSessionTemplate {
      name: None,
      description: None,
      enable_cfs: Some(true),
      cfs: Some(cfs),
      boot_sets: Some(boot_set_vec),
      links: None,
      tenant: None,
    };

    if dry_run {
      log::debug!(
        "Dry run mode: Create BOS sessiontemplate:\n{}",
        serde_json::to_string_pretty(&create_bos_session_template_payload)?
      );

      // Generate a mock name for the BOS session template
      let dry_run_bos_sessiontemplate_name =
        format!("DRYRUN_{}", Uuid::new_v4());
      log::debug!(
        "Dry Run Mode: BOS sessiontemplate name '{dry_run_bos_sessiontemplate_name}' created"
      );
      let mut mock_template = create_bos_session_template_payload.clone();
      mock_template.name = Some(dry_run_bos_sessiontemplate_name);
      bos_st_created_vec.push(mock_template);
    } else {
      let bos_sessiontemplate = crate::ShastaClient::new(
        shasta_base_url,
        shasta_root_cert.to_vec(),
      )?
      .bos_template_v2_put(
        shasta_token,
        &create_bos_session_template_payload,
        &bos_sessiontemplate_name,
      )
      .await?;

      log::debug!(
        "BOS sessiontemplate name '{bos_sessiontemplate_name}' created"
      );

      if bos_sessiontemplate.name.is_none() {
        return Err(Error::SatFile(
          "BOS sessiontemplate API response is missing 'name'".to_string(),
        ));
      }
      bos_st_created_vec.push(bos_sessiontemplate);
    }
  }

  // Create BOS session. Note: reboot operation shuts down the nodes and they may not start
  // up... hence we will split the reboot into 2 operations shutdown and start

  if reboot {
    log::debug!("Rebooting");

    for bos_st in &bos_st_created_vec {
      let bos_st_name = bos_st.name.clone().unwrap_or_default();
      log::debug!(
        "Creating BOS session for BOS sessiontemplate '{bos_st_name}' with action 'reboot'"
      );

      // BOS session v2
      let bos_session = BosSession {
        name: None,
        tenant: None,
        operation: Some(Operation::Reboot),
        template_name: bos_st_name,
        limit: None,
        stage: None,
        include_disabled: None,
        status: None,
        components: None,
      };

      if dry_run {
        log::debug!(
          "Dry run mode: Create BOS session:\n{}",
          serde_json::to_string_pretty(&bos_session)?
        );
      } else {
        let created = crate::ShastaClient::new(
          shasta_base_url,
          shasta_root_cert.to_vec(),
        )?
        .bos_session_v2_post(shasta_token, bos_session)
        .await?;
        bos_sessions_created.push(created);
      }
    }
  }

  // Audit
  let user = common::jwt_ops::get_name(shasta_token)?;
  let username = common::jwt_ops::get_preferred_username(shasta_token)?;

  log::debug!(target: "app::audit", "User: {user} ({username}) ; Operation: Apply cluster");

  Ok((bos_st_created_vec, bos_sessions_created))
}

/// Returns image reference related to a session template in SAT file.
/// An image refenrece can be:
///     - `image_name`
///     - `image_id`
/// Image names are supposed to be fetched using '`get_fuzzy`' function (so we increase the probablity of finding the image in CSM if it was created using 'sat bootprep --overwrite-images') while image ids can be fetched
/// by just 'get' function
/// This function returns a tuple with the image reference and a boolean indicating whether the image is
/// an image id or not
fn get_image_reference_from_bos_sessiontemplate_yaml(
  bos_sessiontemplate_image: &Value,
  ref_name_processed_hashmap: &HashMap<String, String>,
) -> Result<(String, bool), Error> {
  if let Some(bos_sessiontemplate_image_ims) =
    bos_sessiontemplate_image.get("ims")
  {
    // Get boot image to configure the nodes
    if let Some(bos_session_template_image_ims_name) =
      bos_sessiontemplate_image_ims.get("name")
    {
      // BOS sessiontemplate boot image defined by name
      let image_name = bos_session_template_image_ims_name
        .as_str()
        .ok_or_else(|| {
          Error::YamlShape(
            "SAT file: session_template image.ims.name is not a string"
              .to_string(),
          )
        })?
        .to_string();

      Ok((image_name, false))
    } else if let Some(bos_session_template_image_ims_id) =
      bos_sessiontemplate_image_ims.get("id")
    {
      // BOS sessiontemplate boot image defined by id
      let image_id = bos_session_template_image_ims_id
        .as_str()
        .ok_or_else(|| {
          Error::YamlShape(
            "SAT file: session_template image.ims.id is not a string"
              .to_string(),
          )
        })?
        .to_string();

      Ok((image_id, true))
    } else {
      Err(Error::SatFile("neither 'image.ims.name' nor 'image.ims.id' fields defined in session_template.".to_string()))
    }
  } else if let Some(bos_session_template_image_image_ref) =
    bos_sessiontemplate_image.get("image_ref")
  {
    // BOS sessiontemplate boot image defined by image_ref
    let image_ref = bos_session_template_image_image_ref
      .as_str()
      .ok_or_else(|| {
        Error::YamlShape(
          "SAT file: session_template image.image_ref is not a string"
            .to_string(),
        )
      })?
      .to_string();

    let image_id = ref_name_processed_hashmap
      .get(&image_ref)
      .cloned()
      .ok_or_else(|| {
        Error::YamlShape(format!(
          "SAT file: image_ref '{image_ref}' not found in processed image set"
        ))
      })?;

    Ok((image_id, true))
  } else if let Some(image_name_substring) = bos_sessiontemplate_image.as_str()
  {
    let image_name = image_name_substring;
    // Backward compatibility
    // Get base image details

    Ok((image_name.to_string(), false))
  } else {
    Err(Error::SatFile("neither 'image.ims' nor 'image.image_ref' nor 'image.<image id>' sections found in session_template.image.\nExit".to_string()))
  }
}

async fn get_image_details_from_bos_sessiontemplate_yaml(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  image_reference: &str,
  is_image_id: bool,
) -> Result<ims::image::http_client::types::Image, Error> {
  if is_image_id {
    crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .ims_image_get(shasta_token, Some(image_reference))
    .await
    .and_then(|image_vec| {
      image_vec
        .first()
        .cloned()
        .ok_or_else(|| Error::ImageNotFound(image_reference.to_string()))
    })
  } else {
    ims::image::utils::try_get_by_name(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      image_reference,
      Some(&1),
    )
    .await
    .and_then(|image_vec| {
      image_vec
        .first()
        .cloned()
        .ok_or_else(|| Error::ImageNotFound(image_reference.to_string()))
    })
  }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn get_base_image_id_from_sat_file_image_yaml(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  // image_yaml: &Value,
  image_yaml: &image::Image,
  _ref_name_image_id_hashmap: &HashMap<String, String>,
  cray_product_catalog: &BTreeMap<String, String>,
  image_name: &str,
  dry_run: bool,
) -> Result<String, Error> {
  // Get/process base image
  // if let Some(sat_file_image_ims_value_yaml) = image_yaml.get("ims") {
  let base_image_id: String = if let image::BaseOrIms::Ims { ims } =
    &image_yaml.base_or_ims
  {
    // ----------- BASE IMAGE - BACKWARD COMPATIBILITY WITH PREVIOUS SAT FILE
    log::debug!(
      "SAT file - 'image.ims' job ('images' section in SAT file is outdated - switching to backward compatibility)"
    );

    process_sat_file_image_old_version_struct(ims)?
  // } else if let Some(sat_file_image_base_value_yaml) = image_yaml.get("base") {
  } else if let image::BaseOrIms::Base { base } = &image_yaml.base_or_ims {
    /* if let Some(sat_file_image_base_image_ref_value_yaml) =
      sat_file_image_base_value_yaml.get("image_ref")
    { */
    if let image::Base::ImageRef { image_ref } = base {
      log::debug!("SAT file - 'image.base.image_ref' job");

      image_ref.clone()
    /* } else if let Some(sat_file_image_base_ims_value_yaml) =
      sat_file_image_base_value_yaml.get("ims")
    { */
    } else if let image::Base::Ims { ims } = base {
      if let image::ImageBaseIms::NameType { name, r#type } = ims {
        log::debug!("SAT file - 'image.base.ims' job");
        if r#type == "recipe" {
          log::debug!("SAT file - 'image.base.ims' job of type 'recipe'");

          process_sat_file_image_ims_type_recipe(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            name,
            image_name,
            dry_run,
          )
          .await?
        } else {
          return Err(Error::SatFile(
            "Can't process SAT file 'images.base.ims' is missing. Exit"
              .to_string(),
          ));
        }
      } else if let image::Base::Ims { ims } = base {
        #[allow(clippy::collapsible_match)]
        if let image::ImageBaseIms::IdType { id, r#type } = ims {
          if r#type == "image" {
            log::debug!("SAT file - 'image.base.ims' job of type 'image'");

            id.clone()
          } else {
            return Err(Error::SatFile(
              "Can't process SAT file 'images.base.ims' is missing. Exit"
                .to_string(),
            ));
          }
        } else {
          return Err(Error::SatFile(
            "Can't process SAT file 'images.base.ims' is missing. Exit"
              .to_string(),
          ));
        }
      } else {
        return Err(Error::SatFile(
          "Can't process SAT file 'images.base.ims' is missing. Exit"
            .to_string(),
        ));
      }
    // ----------- BASE IMAGE - CRAY PRODUCT CATALOG
    /* } else if let Some(sat_file_image_base_product_value_yaml) =
      sat_file_image_base_value_yaml.get("product")
    { */
    } else if let image::Base::Product { product } = base {
      log::debug!("SAT file - 'image.base.product' job");
      // Base image created from a cray product
      let product_name = &product.name;

      let product_version = product.version.as_ref().ok_or_else(|| {
        Error::YamlShape(format!(
          "SAT file: image base.product '{product_name}' is missing 'version'"
        ))
      })?;

      let product_type = &product.r#type;

      let product_image_map = serde_yaml::from_str::<serde_json::Value>(
        &cray_product_catalog[product_name],
      )?[product_version][product_type]
        .as_object()
        .cloned()
        .ok_or_else(|| {
          Error::SatFile(format!(
            "Cray product catalog: '{product_name}.{product_version}.{product_type}' is missing or not an object"
          ))
        })?;

      let image_id = if let Some(filter) = product.filter.as_ref() {
        filter_product_catalog_images(
          filter,
          product_image_map.clone(),
          image_name,
        )?
      } else {
        // There is no 'image.product.filter' value defined in SAT file. Check Cray
        // product catalog only has 1 image. Othewise fail
        log::debug!(
          "No 'image.product.filter' defined in SAT file. Checking Cray product catalog only/must have 1 image"
        );
        product_image_map
          .values()
          .next()
          .and_then(|value| value.get("id"))
          .and_then(serde_json::Value::as_str)
          .map(str::to_string)
          .ok_or_else(|| {
            Error::SatFile(format!(
              "Cray product catalog: '{product_name}.{product_version}.{product_type}' has no entries with an 'id' field"
            ))
          })?
      };

      // ----------- BASE IMAGE - CRAY PRODUCT CATALOG TYPE RECIPE
      if product_type == "recipes" {
        // Create base image from an IMS job (the 'id' field in
        // images[].base.product.id is the id of the IMS recipe used to
        // build the new base image)

        log::debug!("SAT file - 'image.base.product' job based on IMS recipes");

        let product_recipe_id = image_id.clone();

        process_sat_file_image_product_type_ims_recipe(
          shasta_token,
          shasta_base_url,
          shasta_root_cert,
          &product_recipe_id,
          image_name,
          dry_run,
        )
        .await?

        // ----------- BASE IMAGE - CRAY PRODUCT CATALOG TYPE IMAGE
      } else if product_type == "images" {
        // Base image already created and its id is available in the Cray
        // product catalog

        log::debug!("SAT file - 'image.base.product' job based on IMS images");

        log::debug!("Getting base image id from Cray product catalog");

        image_id
      } else {
        return Err(Error::SatFile(
          "Can't process SAT file, field 'images.base.product.type' must be either 'images' or 'recipes'. Exit".to_string(),
        ));
      }
    } else {
      return Err(Error::SatFile(
        "Can't process SAT file 'images.base.product' is missing. Exit"
          .to_string(),
      ));
    }
  } else {
    return Err(Error::SatFile(
      "Can't process SAT file 'images.base' is missing. Exit".to_string(),
    ));
  };

  Ok(base_image_id)
}
