//! Entry-point function for the apply-SAT-file workflow.

use std::{
  collections::{BTreeMap, HashMap},
  time::Instant,
};

use serde_yaml::Value;

use crate::{
  bos::{BosSession, BosSessionTemplate},
  cfs::v2::CfsConfigurationResponse,
  commands::{
    apply_hw_cluster_pin,
    i_apply_sat_file::utils::{self, SatFile},
  },
  common::kubernetes::{self},
  error::Error,
  hsm::group::utils::update_hsm_group_members,
  ims::Image as ImsImage,
};

/// Borrowed bundle of connection, auth, and feature-flag inputs shared
/// across every phase of `apply_sat_file`. Replaces what used to be 17
/// individually-threaded `exec` arguments propagating through the
/// `gather`/`validate`/`process_*` helpers; per-phase state (live CSM
/// snapshots, the parsed SAT file, mutable ref-name maps) is still
/// passed separately so the context can stay immutable and trivially
/// shareable across `await` points.
struct SatApplyContext<'a> {
  shasta_token: &'a str,
  shasta_base_url: &'a str,
  shasta_root_cert: &'a [u8],
  vault_base_url: &'a str,
  site_name: &'a str,
  k8s_api_url: &'a str,
  gitea_base_url: &'a str,
  gitea_token: &'a str,
  hsm_group_available_vec: &'a [String],
  ansible_verbosity: Option<u8>,
  ansible_passthrough: Option<&'a str>,
  reboot: bool,
  watch_logs: bool,
  timestamps: bool,
  debug_on_failure: bool,
  overwrite: bool,
  dry_run: bool,
}

/// Apply a SAT (System Admin Toolkit) template file against a Shasta system.
///
/// Parses `sat_template_file_yaml`, validates each section against the
/// current state of CSM and the HPE Cray product catalog (read from
/// Kubernetes), and then realises the file by:
///
/// 1. Applying any `hardware` patterns to HSM groups (either component
///    patterns via [`apply_hw_cluster_pin`] or explicit `nodespattern`
///    membership updates).
/// 2. Creating CFS configurations for each entry in `configurations`.
/// 3. Importing every image in `images` (building it through IMS/CFS).
/// 4. Creating BOS session templates from `session_templates`, optionally
///    rebooting the targeted nodes.
///
/// # Arguments
///
/// - `sat_template_file_yaml` — the parsed SAT file as YAML.
/// - `hsm_group_available_vec` — HSM groups the caller is allowed to
///   target; used to reject SAT files that reference out-of-scope groups.
/// - `shasta_k8s_secrets` / `k8s_api_url` — credentials for the in-cluster
///   `cray-product-catalog` `ConfigMap` lookup.
/// - `dry_run` — when `true`, validates and logs the intended actions
///   without mutating CSM.
/// - `overwrite` — replace existing CFS configurations / images with the
///   same name instead of failing.
/// - `reboot` — after creating BOS session templates, also reboot the
///   target nodes through them.
///
/// # Returns
///
/// `(configurations, images, session_templates, sessions)` — the
/// artifacts created from each section of the SAT file. In `dry_run`
/// mode the same tuple is returned populated with the artifacts that
/// *would* have been created. `sessions` is empty unless `reboot` is
/// `true`.
///
/// # Errors
///
/// Returns [`Error`] if the SAT file is malformed, validation against the
/// live CSM state fails, or any underlying API call (CFS, IMS, BOS, HSM,
/// Kubernetes) fails.
///
/// When `watch_logs` is true the CFS-session container logs are
/// streamed line-by-line through `log::info!`; output is routed by the
/// caller's `log` backend rather than written directly to stdout.
#[allow(clippy::too_many_arguments)]
pub async fn exec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  vault_base_url: &str,
  site_name: &str,
  k8s_api_url: &str,
  shasta_k8s_secrets: serde_json::Value,
  sat_template_file_yaml: serde_yaml::Value,
  hsm_group_available_vec: &[String],
  ansible_verbosity_opt: Option<u8>,
  ansible_passthrough_opt: Option<&str>,
  gitea_base_url: &str,
  gitea_token: &str,
  reboot: bool,
  watch_logs: bool,
  timestamps: bool,
  debug_on_failure: bool,
  overwrite: bool,
  dry_run: bool,
) -> Result<
  (
    Vec<CfsConfigurationResponse>,
    Vec<ImsImage>,
    Vec<BosSessionTemplate>,
    Vec<BosSession>,
  ),
  Error,
> {
  let ctx = SatApplyContext {
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    vault_base_url,
    site_name,
    k8s_api_url,
    gitea_base_url,
    gitea_token,
    hsm_group_available_vec,
    ansible_verbosity: ansible_verbosity_opt,
    ansible_passthrough: ansible_passthrough_opt,
    reboot,
    watch_logs,
    timestamps,
    debug_on_failure,
    overwrite,
    dry_run,
  };

  // GET DATA
  //
  // Parse the SAT file and fetch the live CSM / k8s state it is validated
  // against.
  let (sat_file, cray_product_catalog, configuration_vec, image_vec, ims_recipe_vec) =
    gather_sat_apply_data(
      &ctx,
      shasta_k8s_secrets,
      &sat_template_file_yaml,
    )
    .await?;

  // VALIDATION
  //
  // Validate the SAT file sections against the live CSM state.
  validate_sat_file_sections(
    &ctx,
    &sat_file,
    &cray_product_catalog,
    image_vec,
    configuration_vec,
    ims_recipe_vec,
  )
  .await?;

  // PROCESS SAT FILE
  //
  // Process "hardware" / "clusters" section in SAT file
  process_hardware_section(&ctx, &sat_file).await?;

  // Process "configurations" section in SAT file
  let cfs_configurations_created = process_configurations_section(
    &ctx,
    &cray_product_catalog,
    &sat_template_file_yaml,
  )
  .await?;

  // Process "images" section in SAT file
  //
  log::info!("Process images section in SAT file");
  let image_struct_vec = sat_file.images.as_deref().unwrap_or_default();
  // List of image.ref_name already processed
  let mut ref_name_processed_hashmap: HashMap<String, String> = HashMap::new();

  let images_created: Vec<ImsImage> =
    utils::i_import_images_section_in_sat_file(
      ctx.shasta_token,
      ctx.shasta_base_url,
      ctx.shasta_root_cert,
      ctx.vault_base_url,
      ctx.site_name,
      ctx.k8s_api_url,
      &mut ref_name_processed_hashmap,
      image_struct_vec,
      &cray_product_catalog,
      ctx.ansible_verbosity,
      ctx.ansible_passthrough,
      ctx.debug_on_failure,
      ctx.dry_run,
      ctx.watch_logs,
      ctx.timestamps,
    )
    .await?;

  log::info!(
    "Images created: {:?}",
    images_created
      .iter()
      .filter_map(|i| i.id.as_deref())
      .collect::<Vec<&str>>()
  );

  // Process "session_templates" section in SAT file
  //
  log::info!("Process session_template section in SAT file");
  let (sessiontemplates_created, bos_sessions_created) =
    utils::process_session_template_section_in_sat_file(
      ctx.shasta_token,
      ctx.shasta_base_url,
      ctx.shasta_root_cert,
      ref_name_processed_hashmap,
      ctx.hsm_group_available_vec,
      sat_template_file_yaml,
      ctx.reboot,
      ctx.dry_run,
    )
    .await?;

  Ok((
    cfs_configurations_created,
    images_created,
    sessiontemplates_created,
    bos_sessions_created,
  ))
}

/// Parse the SAT file into a [`SatFile`] and fetch the live state it is
/// validated against: the `cray-product-catalog` `ConfigMap` from Kubernetes
/// and the current CFS configurations, IMS images and IMS recipes from CSM.
async fn gather_sat_apply_data(
  ctx: &SatApplyContext<'_>,
  shasta_k8s_secrets: serde_json::Value,
  sat_template_file_yaml: &serde_yaml::Value,
) -> Result<
  (
    SatFile,
    BTreeMap<String, String>,
    Vec<CfsConfigurationResponse>,
    Vec<ImsImage>,
    Vec<crate::ims::recipe::types::RecipeGetResponse>,
  ),
  Error,
> {
  // Get k8s credentials needed to check HPE/Cray product catalog in k8s
  let kube_client = kubernetes::get_client(
    ctx.k8s_api_url,
    shasta_k8s_secrets,
  )
  .await?;

  // Get HPE product catalog from k8s
  let cray_product_catalog =
    kubernetes::try_get_configmap(
      kube_client,
      crate::common::kubernetes::CRAY_PRODUCT_CATALOG_CONFIGMAP,
    )
    .await?;

  // Get data from CSM
  let start = Instant::now();
  log::info!("Fetching data from the backend...");
  let shasta_client = crate::ShastaClient::new(
    ctx.shasta_base_url,
    ctx.shasta_root_cert.to_vec(),
  )?;
  let (configuration_vec, image_vec, ims_recipe_vec) = tokio::try_join!(
    shasta_client.cfs_configuration_v2_get_all(ctx.shasta_token),
    shasta_client.ims_image_get_all(ctx.shasta_token),
    shasta_client.ims_recipe_get(ctx.shasta_token, None),
  )?;

  let duration = start.elapsed();
  log::info!(
    "Time elapsed to fetch information from backend: {duration:?}"
  );

  let sat_file: SatFile =
    serde_yaml::from_str(&serde_yaml::to_string(sat_template_file_yaml)?)?;

  Ok((
    sat_file,
    cray_product_catalog,
    configuration_vec,
    image_vec,
    ims_recipe_vec,
  ))
}

/// Validate the `configurations`, `images` and `session_templates` sections of
/// the SAT file against the live CSM state.
///
/// `image_vec` / `configuration_vec` / `ims_recipe_vec` are the live CSM
/// snapshots; they are consumed here (forwarded to the images validator).
async fn validate_sat_file_sections(
  ctx: &SatApplyContext<'_>,
  sat_file: &SatFile,
  cray_product_catalog: &BTreeMap<String, String>,
  image_vec: Vec<ImsImage>,
  configuration_vec: Vec<CfsConfigurationResponse>,
  ims_recipe_vec: Vec<crate::ims::recipe::types::RecipeGetResponse>,
) -> Result<(), Error> {
  let configuration_struct_vec =
    sat_file.configurations.as_deref().unwrap_or_default();
  let image_struct_vec = sat_file.images.as_deref().unwrap_or_default();
  let bos_session_template_struct_vec =
    sat_file.session_templates.as_deref().unwrap_or_default();

  // Validate 'configurations' section
  utils::validate_sat_file_configurations_section(
    configuration_struct_vec,
    image_struct_vec,
    bos_session_template_struct_vec,
  )?;

  // Validate 'images' section
  utils::validate_sat_file_images_section(
    image_struct_vec,
    configuration_struct_vec,
    ctx.hsm_group_available_vec,
    cray_product_catalog,
    image_vec,
    configuration_vec,
    ims_recipe_vec,
  )?;

  // Validate 'session_template' section
  utils::validate_sat_file_session_template_section(
    ctx.shasta_token,
    ctx.shasta_base_url,
    ctx.shasta_root_cert,
    image_struct_vec,
    configuration_struct_vec,
    bos_session_template_struct_vec,
    ctx.hsm_group_available_vec,
  )
  .await?;

  Ok(())
}

/// Process the `hardware` section of the SAT file: apply component patterns to
/// HSM groups (via [`apply_hw_cluster_pin`]) or update group membership from an
/// explicit `nodespattern`.
async fn process_hardware_section(
  ctx: &SatApplyContext<'_>,
  sat_file: &SatFile,
) -> Result<(), Error> {
  let hardware_patterns = sat_file.hardware.as_deref().unwrap_or_default();
  log::info!("hardware pattern: {hardware_patterns:?}");

  for hw in hardware_patterns {
    let target_hsm_group_name = hw.target.as_str();
    let parent_hsm_group_name = hw.parent.as_str();

    if let Some(pattern) = hw.pattern.as_deref() {
      log::info!(
        "Processing hw component pattern for '{pattern}' for target HSM group '{target_hsm_group_name}' and parent HSM group '{parent_hsm_group_name}'"
      );
      // When applying a SAT file, assume the caller does not want to
      // create new HSM groups or delete empty parent HSM groups (the
      // last three booleans below). This could be made configurable.
      if ctx.dry_run {
        log::info!("Dry run: Create HSM groups based on hardware pattern");
      } else {
        let client = crate::ShastaClient::new(
          ctx.shasta_base_url,
          ctx.shasta_root_cert.to_vec(),
        )?;
        apply_hw_cluster_pin::command::exec(
          &client,
          ctx.shasta_token,
          target_hsm_group_name,
          parent_hsm_group_name,
          pattern,
          true,
          false,
          false,
        )
        .await?;
      }
    } else if let Some(nodes) = hw.nodespattern.as_deref() {
      let hsm_group_members_vec: Vec<String> =
        crate::hsm::group::utils::get_member_vec_from_hsm_name_vec(
          ctx.shasta_token,
          ctx.shasta_base_url,
          ctx.shasta_root_cert,
          &[target_hsm_group_name.to_string()],
        )
        .await?;
      let new_target_hsm_group_members_vec: Vec<String> = nodes
        .split(',')
        .filter(|node| !hsm_group_members_vec.contains(&node.to_string()))
        .map(str::to_string)
        .collect();

      log::info!(
        "Processing new nodes '{nodes}' for target HSM group '{target_hsm_group_name}'",
      );

      if ctx.dry_run {
        log::info!(
          "Dry Run mode: Update HSM group '{target_hsm_group_name}' members to:\n{new_target_hsm_group_members_vec:?}"
        );
      } else {
        update_hsm_group_members(
          ctx.shasta_token,
          ctx.shasta_base_url,
          ctx.shasta_root_cert,
          target_hsm_group_name,
          &hsm_group_members_vec
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
          &new_target_hsm_group_members_vec
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
        )
        .await?;
      }
    }
  }

  Ok(())
}

/// Process the `configurations` section of the SAT file, creating a CFS
/// configuration for each entry and returning the created configurations.
async fn process_configurations_section(
  ctx: &SatApplyContext<'_>,
  cray_product_catalog: &BTreeMap<String, String>,
  sat_template_file_yaml: &serde_yaml::Value,
) -> Result<Vec<CfsConfigurationResponse>, Error> {
  let configuration_yaml_vec_opt = sat_template_file_yaml
    .get("configurations")
    .and_then(Value::as_sequence);

  log::info!("Process configurations section in SAT file");
  let mut cfs_configurations_created: Vec<CfsConfigurationResponse> =
    Vec::new();

  for configuration_yaml in configuration_yaml_vec_opt.unwrap_or(&vec![])
  {
    let cfs_configuration: CfsConfigurationResponse =
      utils::create_cfs_configuration_from_sat_file(
        ctx.shasta_token,
        ctx.shasta_base_url,
        ctx.shasta_root_cert,
        ctx.gitea_base_url,
        ctx.gitea_token,
        cray_product_catalog,
        configuration_yaml,
        ctx.dry_run,
        ctx.site_name,
        ctx.overwrite,
      )
      .await?;

    log::info!("CFS configuration '{}' created", cfs_configuration.name);

    cfs_configurations_created.push(cfs_configuration);
  }

  Ok(cfs_configurations_created)
}

/// Parameters for [`validate_sat_file`].
///
/// Subset of the inputs `apply_sat_file::exec` accepts — only the
/// fields the gather + validate pipeline actually reads. The fields
/// the `process_*` phases need (gitea_*, ansible_*, reboot,
/// watch_logs, timestamps, debug_on_failure, overwrite, dry_run) are
/// filled with defaults inside the wrapper so callers don't have to
/// supply junk values.
pub struct ValidateSatFileParams<'a> {
  /// Shasta API authentication token.
  pub shasta_token: &'a str,
  /// Shasta API base URL.
  pub shasta_base_url: &'a str,
  /// Root CA certificate for validating Shasta TLS connections.
  pub shasta_root_cert: &'a [u8],
  /// Vault base URL for secret retrieval.
  pub vault_base_url: &'a str,
  /// Site name (used for logging and context).
  pub site_name: &'a str,
  /// Kubernetes API URL for in-cluster ConfigMap access.
  pub k8s_api_url: &'a str,
  /// HSM groups the caller is allowed to target.
  pub hsm_group_available_vec: &'a [String],
  /// Parsed SAT template file as YAML.
  pub sat_template_file_yaml: serde_yaml::Value,
}

/// Validate a SAT file against the live CSM state without mutating
/// anything.
///
/// Public entry point that wraps the private `gather_sat_apply_data`
/// + `validate_sat_file_sections` pair: fetches the k8s
/// `cray-product-catalog` ConfigMap and the current CFS / IMS /
/// recipe lists, parses the SAT YAML, and runs the same per-section
/// validators the apply pipeline runs.
///
/// Returns `Ok(())` if the SAT file would apply cleanly given the
/// current CSM state; returns the first validation [`Error`]
/// encountered otherwise (fail-fast — see the design doc).
///
/// `shasta_k8s_secrets` is the Vault-fetched k8s credential blob;
/// taken as a separate argument to mirror `apply_sat_file::exec`'s
/// signature and avoid coupling csm-rs to a Vault client.
pub async fn validate_sat_file(
  params: ValidateSatFileParams<'_>,
  shasta_k8s_secrets: serde_json::Value,
) -> Result<(), Error> {
  // Reuse the existing context struct. Fields not read by the
  // gather + validate path get empty defaults; the validator never
  // reaches the apply phase so these stay inert.
  let ctx = SatApplyContext {
    shasta_token: params.shasta_token,
    shasta_base_url: params.shasta_base_url,
    shasta_root_cert: params.shasta_root_cert,
    vault_base_url: params.vault_base_url,
    site_name: params.site_name,
    k8s_api_url: params.k8s_api_url,
    gitea_base_url: "",
    gitea_token: "",
    hsm_group_available_vec: params.hsm_group_available_vec,
    ansible_verbosity: None,
    ansible_passthrough: None,
    reboot: false,
    watch_logs: false,
    timestamps: false,
    debug_on_failure: false,
    overwrite: false,
    dry_run: true,
  };

  let (sat_file, cray_product_catalog, configuration_vec, image_vec, ims_recipe_vec) =
    gather_sat_apply_data(
      &ctx,
      shasta_k8s_secrets,
      &params.sat_template_file_yaml,
    )
    .await?;

  validate_sat_file_sections(
    &ctx,
    &sat_file,
    &cray_product_catalog,
    image_vec,
    configuration_vec,
    ims_recipe_vec,
  )
  .await
}
