use std::collections::{BTreeMap, HashMap};

use chrono::Local;
use serde_json::Map;
use uuid::Uuid;

use crate::{
  cfs::{
    self,
    v2::{
      Ansible, Artifact, CfsConfigurationResponse, CfsSessionGetResponse,
      CfsSessionPostRequest, Configuration, Group, Session, Status, Target,
    },
  },
  error::Error,
  hsm,
  ims::{self},
};

use crate::common::{
  kubernetes::{self, i_print_cfs_session_logs},
  vault::http_client::fetch_shasta_k8s_secrets_from_vault,
};

use super::{
  configuration,
  image::{self, Filter},
  session_templates::get_base_image_id_from_sat_file_image_yaml,
};

/// Analyze a list of images in SAT file and returns the image to process next.
/// Input values:
///  - `image_yaml_vec`: the list of images in the SAT file, each element is a `serde_yaml::Value`
///  - `ref_name_processed_vec`: he list of images (`ref_name`) already processed
/// Note:
/// `image.base.image_ref` value in SAT file points to the image it depends on (`image.ref_name`)
/// NOTE 2: we assume that there may be a mix of images in SAT file with and without "`ref_name`"
/// value, we will use the function "`get_ref_name`" which will fall back to "name" field if
/// "`ref_name`" is missing in the image
/// An image is ready to be processed if:
///  - It does not depends on another image (`image.base.image_ref` is missing)
///  - The image it depends to is already processed (`image.base.image_ref` included in
///  `ref_name_processed`)
///  - It has not been already processed
/// Analyze a list of images in SAT file and returns the image to process next.
/// Input values:
///  - `image_yaml_vec`: the list of images in the SAT file, each element is a `serde_yaml::Value`
///  - `ref_name_processed_vec`: he list of images (`ref_name`) already processed
/// Note:
/// `image.base.image_ref` value in SAT file points to the image it depends on (`image.ref_name`)
/// NOTE 2: we assume that there may be a mix of images in SAT file with and without "`ref_name`"
/// value, we will use the function "`get_ref_name`" which will fall back to "name" field if
/// "`ref_name`" is missing in the image
/// An image is ready to be processed if:
///  - It does not depends on another image (`image.base.image_ref` is missing)
///  - The image it depends to is already processed (`image.base.image_ref` included in
///  `ref_name_processed`)
///  - It has not been already processed
pub fn get_next_image_in_sat_file_to_process_struct(
  image_yaml_vec: &[image::Image],
  ref_name_processed_vec: &[String],
) -> Option<image::Image> {
  image_yaml_vec
    .iter()
    .find(|image_yaml| {
      let ref_name: &str =
        &get_image_name_or_ref_name_to_process_struct(image_yaml); // Again, because we assume images in
      // SAT file may or may not have ref_name value, we will use "get_ref_name" function to
      // get the id of the image

      /* let image_base_image_ref_opt: Option<&str> = image_yaml
      .get("base")
      .and_then(|image_base_yaml| image_base_yaml.get("image_ref"))
      .and_then(Value::as_str); */

      let image_base_image_ref_opt = if let image::BaseOrIms::Base {
        base: image::Base::ImageRef { image_ref },
      } = &image_yaml.base_or_ims
      {
        Some(image_ref)
      } else {
        None
      };

      !ref_name_processed_vec.contains(&ref_name.to_string())
        && image_base_image_ref_opt.is_none_or(|image_base_image_ref| {
            ref_name_processed_vec.contains(&image_base_image_ref.clone())
          })
    })
    .cloned()
}

/// Get the "`ref_name`" from an image, because we need to be aware of which images in SAT file have
/// been processed in order to find the next image to process. We assume not all images in the yaml
/// will have an "`image_ref`" value, therefore we will use "`ref_name`" or "name" field if the former
/// is missing
pub fn get_image_name_or_ref_name_to_process_struct(
  image_yaml: &image::Image,
) -> String {
  if let Some(ref_name) = image_yaml.ref_name.as_ref() {
    ref_name.clone()
  } else {
    // If the image processed is missing the field "ref_name", then use the field "name"
    // instead, this is needed to flag this image as processed and filtered when
    // calculating the next image to process (get_next_image_to_process)
    image_yaml.name.clone()
  }
}

/// Build every entry in the SAT file's `images` section: import the
/// base recipe / image and run the associated CFS session. When
/// `watch_logs` is true the CFS session's container logs are streamed
/// line-by-line through `log::debug!`.
///
/// Returns only the produced `Image`s. The
/// [`i_create_image_from_sat_file_serde_yaml`] per-image helper now
/// returns a `(Image, CfsSessionGetResponse)` tuple; this bulk path
/// destructures and discards the session because the whole-SAT-file
/// flow does not emit per-image provenance metadata. The single-image
/// flow ([`crate::backend_connector::sat::Csm`]'s `apply_image`) is
/// where metadata stamping is wired up.
#[allow(clippy::too_many_arguments)]
pub async fn i_import_images_section_in_sat_file(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  vault_base_url: &str,
  site_name: &str,
  k8s_api_url: &str,
  ref_name_processed_hashmap: &mut HashMap<String, String>,
  // image_yaml_vec: &[serde_yaml::Value],
  image_yaml_vec: &[image::Image],
  cray_product_catalog: &BTreeMap<String, String>,
  ansible_verbosity_opt: Option<u8>,
  ansible_passthrough_opt: Option<&str>,
  debug_on_failure: bool, // tag: &str,
  dry_run: bool,
  watch_logs: bool,
  timestamps: bool,
) -> Result<Vec<ims::image::http_client::types::Image>, Error> {
  if image_yaml_vec.is_empty() {
    log::warn!("No images found in SAT file. Nothing to process.");
    return Ok(Vec::new());
  }

  // Get an image to process (the image either has no dependency or it's image dependency has
  // already ben processed)
  let mut next_image_to_process_opt: Option<image::Image> =
    get_next_image_in_sat_file_to_process_struct(
      image_yaml_vec,
      &ref_name_processed_hashmap
        .keys()
        .cloned()
        .collect::<Vec<String>>(),
    );

  // Process images
  log::debug!("Processing image '{next_image_to_process_opt:?}'");
  let mut images_created: Vec<ims::image::http_client::types::Image> =
    Vec::new();

  while let Some(image_yaml) = &next_image_to_process_opt {
    let image = i_create_image_from_sat_file_serde_yaml(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      vault_base_url,
      site_name,
      k8s_api_url,
      image_yaml,
      cray_product_catalog,
      ansible_verbosity_opt,
      ansible_passthrough_opt,
      ref_name_processed_hashmap,
      debug_on_failure,
      dry_run,
      watch_logs,
      timestamps,
    )
    .await?;

    let image_id = image.id.clone().unwrap_or_default();

    ref_name_processed_hashmap.insert(
      get_image_name_or_ref_name_to_process_struct(image_yaml),
      image_id,
    );

    images_created.push(image);

    next_image_to_process_opt = get_next_image_in_sat_file_to_process_struct(
      image_yaml_vec,
      &ref_name_processed_hashmap
        .keys()
        .cloned()
        .collect::<Vec<String>>(),
    );
  }

  Ok(images_created)
}

/// Provenance metadata key namespace stamped onto each IMS image
/// after a successful CFS session. Three keys document, on the image
/// itself, the CFS facts that produced it — source/base image id
/// (`META_BASE`), target HSM groups as JSON-encoded array
/// (`META_GROUPS`), and the CFS configuration name (`META_CONFIG`).
/// Manta-side commands read these keys directly off `Image.metadata`.
const META_BASE: &str = "manta.image_session.base";
const META_GROUPS: &str = "manta.image_session.groups";
const META_CONFIG: &str = "manta.image_session.configuration";

/// Build one image entry from a SAT file YAML node: resolve the base
/// (recipe or existing image), create the IMS image, kick off a CFS
/// session, stream its container logs through `log::debug!` if
/// `watch_logs` is on, then call [`stamp_image_session_metadata`] to
/// fill in `manta.image_session.*` and PATCH the image back so the
/// metadata survives the request.
///
/// The stamp + PATCH step is best-effort: a stamp that fails (missing
/// fields on the CFS session) and a PATCH that fails (network / IMS
/// error) are both logged at `warn` and otherwise swallowed. The image
/// itself was built successfully and a missing
/// `manta.image_session.*` annotation can be backfilled.
///
/// In `dry_run` mode no CFS session is created and no PATCH is
/// attempted; the function returns a fake `Image` with a synthetic
/// `DRYRUN_<uuid>` id.
#[allow(clippy::too_many_arguments)]
pub async fn i_create_image_from_sat_file_serde_yaml(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  vault_base_url: &str,
  site_name: &str,
  k8s_api_url: &str,
  // image_yaml: &serde_yaml::Value, // NOTE: image may be an IMS job or a CFS session
  image_yaml: &image::Image,
  cray_product_catalog: &BTreeMap<String, String>,
  ansible_verbosity_opt: Option<u8>,
  ansible_passthrough_opt: Option<&str>,
  ref_name_image_id_hashmap: &HashMap<String, String>,
  _debug_on_failure: bool,
  dry_run: bool,
  watch_logs: bool,
  timestamps: bool,
) -> Result<ims::image::http_client::types::Image, Error> {
  let cfs_session = create_cfs_session_for_sat_image(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    image_yaml,
    cray_product_catalog,
    ansible_verbosity_opt,
    ansible_passthrough_opt,
    ref_name_image_id_hashmap,
    dry_run,
  )
  .await?;

  let cfs_session = wait_or_stream_cfs_session(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    vault_base_url,
    site_name,
    k8s_api_url,
    cfs_session,
    watch_logs,
    timestamps,
    dry_run,
  )
  .await?;

  collect_and_stamp_image(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &cfs_session,
    &image_yaml.name,
    dry_run,
  )
  .await
}

/// Part 1: build the CFS session request from the SAT-file image YAML
/// and create it. In `dry_run` mode returns a synthetic
/// `CfsSessionGetResponse` (`DRYRUN-<uuid>` result id) with no network
/// call. Otherwise POSTs to CFS and returns the just-created session —
/// the response carries only the session name and initial status, so
/// callers must drive it to completion via [`wait_or_stream_cfs_session`].
///
/// Exposed `pub` so the [`crate::backend_connector::sat`]
/// `SatTrait::apply_sat_image_create_session` impl can call this
/// directly when the caller wants to drive the monitor + stamp steps
/// itself (e.g. the manta-cli SAT-image build pipeline) instead of
/// going through [`i_create_image_from_sat_file_serde_yaml`]'s
/// monolithic flow.
#[allow(clippy::too_many_arguments)]
pub async fn create_cfs_session_for_sat_image(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  image_yaml: &image::Image,
  cray_product_catalog: &BTreeMap<String, String>,
  ansible_verbosity_opt: Option<u8>,
  ansible_passthrough_opt: Option<&str>,
  ref_name_image_id_hashmap: &HashMap<String, String>,
  dry_run: bool,
) -> Result<CfsSessionGetResponse, Error> {
  let cfs_session = get_session_from_image_yaml(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    image_yaml,
    ref_name_image_id_hashmap,
    cray_product_catalog,
    ansible_verbosity_opt,
    ansible_passthrough_opt,
    dry_run,
  )
  .await?;

  if dry_run {
    log::debug!(
      "Dry run mode: Create CFS session:\n{}",
      serde_json::to_string_pretty(&cfs_session)?
    );

    let cfs_session_target_group = Group {
      name: "DRYTUN-group_name".to_string(),
      members: vec!["DRYRUN-group_member".to_string()],
    };
    let cfs_session_target = Target {
      definition: Some("image".to_string()),
      groups: Some(vec![cfs_session_target_group]),
    };
    let configuration = Configuration {
      name: Some(cfs_session.configuration_name.clone()),
      limit: cfs_session.configuration_limit.clone(),
    };
    let ansible = Ansible {
      config: cfs_session.ansible_config.clone(),
      limit: cfs_session.ansible_limit.clone(),
      verbosity: cfs_session.ansible_verbosity,
      passthrough: cfs_session.ansible_passthrough.clone(),
    };
    let mut base_image_id_vec: Vec<String> = cfs_session.get_base_image_ids();
    base_image_id_vec.sort();
    base_image_id_vec.dedup();

    let base_image_id = match base_image_id_vec.as_slice() {
      [] => {
        return Err(Error::Message(
          "CFS session must create at least one image".to_string(),
        ));
      }
      [_, _, ..] => return Err(Error::Message(
        "CFS session generated from SAT file cannot build more than one image"
          .to_string(),
      )),
      [a] => a,
    };

    let artifact = Artifact {
      image_id: Some(base_image_id.clone()),
      result_id: Some(format!("DRYRUN-{}", Uuid::new_v4())),
      r#type: None,
    };

    let mock_cfs_session = CfsSessionGetResponse {
      name: cfs_session.name,
      target: Some(cfs_session_target),
      tags: None,
      configuration: Some(configuration),
      ansible: Some(ansible),
      status: Some(Status {
        artifacts: Some(vec![artifact]),
        session: Some(Session {
          job: Some(format!("DRYRUN-{}", Uuid::new_v4())),
          completion_time: Some(Local::now().to_rfc3339()),
          start_time: Some(Local::now().to_rfc3339()),
          status: Some("complete".to_string()),
          succeeded: Some("true".to_string()),
        }),
      }),
    };

    log::debug!(
      "Dry run mode: CFS session created:\n{}",
      serde_json::to_string_pretty(&mock_cfs_session)?
    );

    Ok(mock_cfs_session)
  } else {
    cfs::session::post(
      shasta_token,
      shasta_base_url,
      shasta_root_cert,
      &cfs_session,
    )
    .await
    .map_err(|e| {
      Error::SatFile(format!("Could not create Image. Reason:\n{e}"))
    })
  }
}

/// Part 2: drive a just-POSTed CFS session to completion. When
/// `watch_logs` is true the session's container logs are streamed
/// line-by-line through `log::info!`; either way the function blocks
/// until the session finishes and returns the final
/// `CfsSessionGetResponse`. Errors with `Error::SatFile` if the
/// session ends without `is_success()`.
///
/// In `dry_run` mode no waiting happens — the input session is
/// returned unchanged (it was already mocked as "complete" by
/// [`create_cfs_session_for_sat_image`]).
#[allow(clippy::too_many_arguments)]
async fn wait_or_stream_cfs_session(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  vault_base_url: &str,
  site_name: &str,
  k8s_api_url: &str,
  cfs_session: CfsSessionGetResponse,
  watch_logs: bool,
  timestamps: bool,
  dry_run: bool,
) -> Result<CfsSessionGetResponse, Error> {
  if dry_run {
    return Ok(cfs_session);
  }

  let cfs_session_name = cfs_session.name.clone();

  if watch_logs {
    log::info!("Fetching logs form CFS session {cfs_session_name} ...");
    let shasta_k8s_secrets = fetch_shasta_k8s_secrets_from_vault(
      vault_base_url,
      shasta_token,
      site_name,
    )
    .await?;

    let client =
      kubernetes::get_client(k8s_api_url, shasta_k8s_secrets)
        .await?;

    i_print_cfs_session_logs(client, &cfs_session_name, timestamps).await?;
  }

  cfs::session::utils::wait_cfs_session_to_finish(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &cfs_session_name,
  )
  .await?;

  let cfs_session = cfs::session::get_one(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &cfs_session_name,
  )
  .await?;

  if !cfs_session.is_success() {
    return Err(Error::SatFile(format!(
      "CFS session '{}' failed. Exit",
      cfs_session.name
    )));
  }

  Ok(cfs_session)
}

/// Part 3: fetch the IMS image produced by the (already-complete) CFS
/// session, stamp `manta.image_session.{base,groups,configuration}`
/// provenance onto it, and best-effort PATCH the metadata back to IMS.
///
/// PATCH failure is logged at `warn` and otherwise swallowed — the
/// image itself was built and the metadata can be backfilled.
///
/// In `dry_run` mode no IMS calls happen: a placeholder `Image` is
/// constructed in-memory, stamped, and returned.
///
/// Exposed `pub` so the [`crate::backend_connector::sat`]
/// `SatTrait::apply_sat_image_stamp_from_session` impl can call this
/// directly after its own `cfs::session::get_one` lookup, letting the
/// stamp run as a separate HTTP step rather than buried inside
/// [`i_create_image_from_sat_file_serde_yaml`].
pub async fn collect_and_stamp_image(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  cfs_session: &CfsSessionGetResponse,
  image_name: &str,
  dry_run: bool,
) -> Result<ims::image::http_client::types::Image, Error> {
  let image_id = cfs_session.first_result_id().ok_or_else(|| {
    Error::Message(format!(
      "CFS session '{}' produced no result image id",
      cfs_session.name
    ))
  })?;

  if dry_run {
    let mut image = ims::image::http_client::types::Image {
      id: Some(image_id.to_string()),
      name: image_name.to_string(),
      ..Default::default()
    };
    stamp_image_session_metadata(&mut image, cfs_session);

    log::debug!(
      "Dry run mode: Image created:\n{}",
      serde_json::to_string_pretty(&image)?
    );

    return Ok(image);
  }

  log::debug!("Image '{image_name}' ({image_id}) created");

  let client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let mut image = client
    .ims_image_get(shasta_token, Some(image_id))
    .await?
    .into_iter()
    .next()
    .ok_or_else(|| Error::ImageNotFound(image_id.to_string()))?;

  if stamp_image_session_metadata(&mut image, cfs_session) {
    let patch = ims::image::http_client::types::PatchImage {
      metadata: image.metadata.clone(),
      ..Default::default()
    };

    let image_id_for_patch = image.id.clone().unwrap_or_default();
    if let Err(e) = client
      .ims_image_patch(shasta_token, &image_id_for_patch, &patch)
      .await
    {
      log::warn!(
        "image_session metadata PATCH failed for image \
         {image_id_for_patch}: {e}; image built but provenance not \
         persisted",
      );
    }
  }

  Ok(image)
}

/// Stamp `manta.image_session.{base,groups,configuration}` onto
/// `image.metadata` from the finished CFS session.
///
/// Pure in-memory mutation; the caller is responsible for `PATCHing`
/// `image` back to IMS so the metadata survives the request.
///
/// Returns `true` when all three keys were written, `false` when any
/// required field on the session was missing or unserialisable (a
/// `warn` line names the missing field). The `false` path is the
/// signal to the caller that there is nothing new to PATCH.
fn stamp_image_session_metadata(
  image: &mut ims::image::http_client::types::Image,
  cfs_session: &cfs::v2::CfsSessionGetResponse,
) -> bool {
  let image_id_for_log = image.id.as_deref().unwrap_or("<no id>").to_string();

  // csm-rs's CfsSessionGetResponse stores the base image id as the
  // first member of the first target group (see
  // `CfsSessionPostRequest::new` for the construction side). The CFS
  // v2 wire format also exposes it via `target.image_map[*].source_id`
  // but csm-rs's internal Target type doesn't deserialize that field.
  let Some(base) = cfs_session
    .target
    .as_ref()
    .and_then(|t| t.groups.as_ref())
    .and_then(|g| g.first())
    .and_then(|g| g.members.first())
    .cloned()
  else {
    log::warn!(
      "CFS session for image {image_id_for_log} has no \
       target.groups[0].members[0]; skipping image_session metadata stamp",
    );
    return false;
  };

  let Some(configuration) = cfs_session.configuration_name() else {
    log::warn!(
      "CFS session for image {image_id_for_log} has no \
       configuration.name; skipping image_session metadata stamp",
    );
    return false;
  };
  let configuration = configuration.to_string();

  let groups = cfs_session.get_target_hsm().unwrap_or_default();
  let groups_json = match serde_json::to_string(&groups) {
    Ok(s) => s,
    Err(e) => {
      log::warn!(
        "could not JSON-encode HSM groups {groups:?} for image \
         {image_id_for_log}: {e}; skipping image_session metadata stamp",
      );
      return false;
    }
  };

  let metadata = image.metadata.get_or_insert_with(HashMap::new);
  metadata.insert(META_BASE.into(), base);
  metadata.insert(META_GROUPS.into(), groups_json);
  metadata.insert(META_CONFIG.into(), configuration);
  true
}

#[allow(clippy::too_many_arguments)]
async fn get_session_from_image_yaml(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  // image_yaml: Value,
  image_yaml: &image::Image,
  ref_name_image_id_hashmap: &HashMap<String, String>,
  cray_product_catalog: &BTreeMap<String, String>,
  ansible_verbosity_opt: Option<u8>,
  ansible_passthrough_opt: Option<&str>,
  dry_run: bool,
) -> Result<CfsSessionPostRequest, Error> {
  // Collect CFS session details from SAT file
  // Get CFS image name from SAT file
  let image_name = image_yaml.name.clone();

  log::debug!(
    "Creating CFS session related to build image '{image_name}'"
  );

  // Get CFS configuration related to CFS session in SAT file
  let configuration_name =
    image_yaml.configuration.as_ref().ok_or_else(|| {
      Error::YamlShape(format!(
        "SAT file: image '{}' is missing 'configuration' field",
        image_yaml.name
      ))
    })?;

  // Get HSM groups related to CFS session in SAT file
  let groups_name: Vec<&str> = image_yaml
    .configuration_group_names
    .as_ref()
    .map(|group_name_vec| group_name_vec.iter().map(String::as_str).collect())
    .unwrap_or_default();

  // VALIDATION: make sure grups in SAT.images "CFS session" are valid
  // NOTE: this is temporary until we get rid off "group" names as ansible folder names
  let invalid_groups: Vec<String> =
    hsm::group::hacks::validate_groups_auth_token(&groups_name, shasta_token)?;

  if !invalid_groups.is_empty() {
    log::debug!("CFS session group validation - failed");

    return Err(Error::SatFile(format!(
      "Please fix 'images' section in SAT file.\nInvalid groups: {invalid_groups:?}"
    )));
  }
  log::debug!("CFS session group validation - passed");

  let base_image_id = get_base_image_id_from_sat_file_image_yaml(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    image_yaml,
    ref_name_image_id_hashmap,
    cray_product_catalog,
    &image_name,
    dry_run,
  )
  .await?;

  // Create a CFS session
  log::debug!("Creating CFS session");

  // Create CFS session
  let session_name = image_name.clone();

  let cfs_session = CfsSessionPostRequest::new(
    session_name,
    configuration_name,
    None,
    ansible_verbosity_opt,
    ansible_passthrough_opt,
    true,
    Some(&groups_name),
    Some(&base_image_id),
  );

  Ok(cfs_session)
}

pub(super) async fn process_sat_file_image_product_type_ims_recipe(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  recipe_id: &str,
  image_name: &str,
  dry_run: bool,
) -> Result<String, Error> {
  let root_ims_key_name = "mgmt root key";

  // Get root public ssh key
  let root_public_ssh_key = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .ims_public_keys_v3_get_single(shasta_token, root_ims_key_name)
  .await?
  .ok_or_else(|| Error::ImsKeyNotFound(root_ims_key_name.to_string()))?;

  let root_public_ssh_key_id = root_public_ssh_key.id.ok_or_else(|| {
    Error::Message(
      "IMS public-key response missing server-generated 'id'".to_string(),
    )
  })?;

  // let ims_job = ims::job::types::JobPostRequest {
  let ims_job = ims::job::types::Job {
    job_type: "create".to_string(),
    image_root_archive_name: image_name.to_string(),
    kernel_file_name: Some("vmlinuz".to_string()),
    initrd_file_name: Some("initrd".to_string()),
    kernel_parameters_file_name: Some("kernel-parameters".to_string()),
    artifact_id: recipe_id.to_string(),
    public_key_id: root_public_ssh_key_id,
    ssh_containers: None, // Should this be None ???
    enable_debug: Some(false),
    build_env_size: Some(15),
    require_dkms: None, // FIXME: check SAT file and see if this value needs to be set
    id: None,
    created: None,
    status: None,
    kubernetes_job: None,
    kubernetes_service: None,
    kubernetes_configmap: None,
    resultant_image_id: None,
    kubernetes_namespace: None,
    arch: None,
  };

  let ims_job = if dry_run {
    log::debug!(
      "Dry run mode: Create IMS job:\n{}",
      serde_json::to_string_pretty(&ims_job)?
    );
    let mut dry_run_ims_job = ims_job;
    dry_run_ims_job.resultant_image_id = Some(Uuid::new_v4().to_string());
    dry_run_ims_job
  } else {
    crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .ims_job_post_sync(shasta_token, &ims_job)
    .await?
  };

  ims_job.resultant_image_id.ok_or_else(|| {
    Error::SatFile(format!(
      "IMS job for image '{image_name}' did not produce a resultant_image_id"
    ))
  })
}

pub(super) async fn process_sat_file_image_ims_type_recipe(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  recipe_name: &str,
  image_name: &str,
  dry_run: bool,
) -> Result<String, Error> {
  // Base image needs to be created from a IMS job using an IMS recipe
  // Get all IMS recipes
  let recipe_detail_vec: Vec<ims::recipe::types::RecipeGetResponse> =
    crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .ims_recipe_get(shasta_token, None)
    .await?;

  // Filter recipes by name
  let recipe_detail_opt = recipe_detail_vec
    .iter()
    .find(|recipe| recipe.name == recipe_name);

  log::debug!("IMS recipe details:\n{recipe_detail_opt:#?}");

  // Check recipe with requested name exists
  let recipe_detail = recipe_detail_opt.ok_or_else(|| {
    Error::SatFile(format!(
      "IMS recipe with name '{recipe_name}' - not found. Exit"
    ))
  })?;
  let recipe_id = recipe_detail.id.as_ref().ok_or_else(|| {
    Error::SatFile(format!("IMS recipe '{recipe_name}' has no 'id' field"))
  })?;

  log::debug!("IMS recipe id found '{recipe_id}'");

  let root_ims_key_name = "mgmt root key";

  // Get root public ssh key
  let root_public_ssh_key = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .ims_public_keys_v3_get_single(shasta_token, root_ims_key_name)
  .await?
  .ok_or_else(|| Error::ImsKeyNotFound(root_ims_key_name.to_string()))?;

  let root_public_ssh_key_id = root_public_ssh_key.id.ok_or_else(|| {
    Error::Message(
      "IMS public-key response missing server-generated 'id'".to_string(),
    )
  })?;

  let ims_job = ims::job::types::Job {
    job_type: "create".to_string(),
    image_root_archive_name: image_name.to_string(),
    kernel_file_name: Some("vmlinuz".to_string()),
    initrd_file_name: Some("initrd".to_string()),
    kernel_parameters_file_name: Some("kernel-parameters".to_string()),
    artifact_id: recipe_id.clone(),
    public_key_id: root_public_ssh_key_id,
    ssh_containers: None, // Should this be None ???
    enable_debug: Some(false),
    build_env_size: Some(15),
    require_dkms: None, // FIXME: check SAT file and see if this value needs to be set
    id: None,
    created: None,
    status: None,
    kubernetes_job: None,
    kubernetes_service: None,
    kubernetes_configmap: None,
    resultant_image_id: None,
    kubernetes_namespace: None,
    arch: None,
  };

  let ims_job = if dry_run {
    log::debug!(
      "Dry run mode: Create IMS job:\n{}",
      serde_json::to_string_pretty(&ims_job)?
    );
    ims_job
  } else {
    crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .ims_job_post_sync(shasta_token, &ims_job)
    .await?
  };

  log::debug!("IMS job response:\n{ims_job:#?}");

  ims_job.resultant_image_id.ok_or_else(|| {
    Error::SatFile(format!(
      "IMS job for image '{image_name}' did not produce a resultant_image_id"
    ))
  })
}

pub(super) fn process_sat_file_image_old_version_struct(
  sat_file_image_ims_value_yaml: &image::ImageIms,
) -> Result<String, Error> {
  if let image::ImageIms::IdIsRecipe {
    id,
    is_recipe: false,
  } = &sat_file_image_ims_value_yaml
  {
    Ok(id.clone())
  } else {
    Err(Error::SatFile("Functionality not built. Exit".to_string()))
  }
}

/// Apply a SAT-file `Filter` (prefix/wildcard rules) to a list of
/// product catalog images and return the matching ones.
pub fn filter_product_catalog_images(
  filter: &Filter,
  image_map: Map<String, serde_json::Value>,
  image_name: &str,
) -> Result<String, Error> {
  if let Filter::Arch { arch } = filter {
    // Search image in product catalog and filter by arch
    let image_key_vec = image_map
      .keys()
      .collect::<Vec<_>>()
      .into_iter()
      .filter(|product| product.split('.').next_back().eq(&Some(arch.as_ref())))
      .collect::<Vec<_>>();

    if image_key_vec.is_empty() {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' not found. Exit"
      )))
    } else if image_key_vec.len() > 1 {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' multiple items found. Exit"
      )))
    } else {
      let image_key: &String = image_key_vec[0];
      image_map
        .get(image_key)
        .and_then(|image_value| image_value.get("id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
          Error::CrayProductCatalog(format!(
            "Product catalog entry '{image_key}' for image '{image_name}' has no 'id' field"
          ))
        })
    }
  // } else if let Some(wildcard) = filter.get("wildcard") {
  } else if let Filter::Wildcard { wildcard } = filter {
    // Search image in product catalog and filter by wildcard
    let image_key_vec = image_map
      .keys()
      .filter(|product| product.contains(wildcard.as_str()))
      .collect::<Vec<_>>();

    if image_key_vec.is_empty() {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' not found. Exit"
      )))
    } else if image_key_vec.len() > 1 {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' multiple items found. Exit"
      )))
    } else {
      let image_key = image_key_vec[0];
      image_map
        .get(image_key)
        .and_then(|image_value| image_value.get("id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
          Error::CrayProductCatalog(format!(
            "Product catalog entry '{image_key}' for image '{image_name}' has no 'id' field"
          ))
        })
    }
  // } else if let Some(prefix) = filter.get("prefix") {
  } else if let Filter::Prefix { prefix } = filter {
    // Search image in product catalog and filter by prefix
    let image_key_vec = image_map
      .keys()
      .filter(|product| product.strip_prefix(prefix.as_str()).is_some())
      .collect::<Vec<_>>();

    if image_key_vec.is_empty() {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' not found. Exit"
      )))
    } else if image_key_vec.len() > 1 {
      Err(Error::CrayProductCatalog(format!(
        "Product catalog for image '{image_name}' multiple items found. Exit"
      )))
    } else {
      let image_key = image_key_vec[0];
      image_map
        .get(image_key)
        .and_then(|image_value| image_value.get("id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
          Error::CrayProductCatalog(format!(
            "Product catalog entry '{image_key}' for image '{image_name}' has no 'id' field"
          ))
        })
    }
  } else {
    Err(Error::CrayProductCatalog(format!(
      "Product catalog for image '{image_name}' not found. Exit"
    )))
  }
}

/// Pre-flight validation for a SAT file's `images` section: rejects
/// entries that reference unknown configurations, out-of-scope HSM
/// groups, or unavailable product catalog images.
pub fn validate_sat_file_images_section(
  image_yaml_vec: &[image::Image],
  configuration_yaml_vec: &[configuration::Configuration],
  hsm_group_available_vec: &[String],
  cray_product_catalog: &BTreeMap<String, String>,
  image_vec: Vec<ims::image::http_client::types::Image>,
  configuration_vec: Vec<CfsConfigurationResponse>,
  ims_recipe_vec: Vec<ims::recipe::types::RecipeGetResponse>,
) -> Result<(), Error> {
  // Validate 'images' section in SAT file

  for image_yaml in image_yaml_vec {
    // Validate image
    let image_name = &image_yaml.name;

    log::debug!("Validate 'image' '{image_name}'");

    if let image::BaseOrIms::Ims { ims } = &image_yaml.base_or_ims {
      if let image::ImageIms::IdIsRecipe { id, is_recipe: _ } = ims {
        // Validate base image
        log::debug!("Validate 'image' '{image_name}' base image '{id}'");

        // Old format
        log::debug!(
          "Searching image.ims.id (old format - backward compatibility) '{id}' in CSM",
        );

        let is_image_base_id_in_csm = image_vec.iter().any(
          |image: &ims::image::http_client::types::Image| {
            image.id.as_ref().is_some_and(|image_id| image_id.eq(id))
          },
        );

        if !is_image_base_id_in_csm {
          return Err(Error::SatFile(format!(
            "Could not find base image id '{}' in image '{}'. Exit",
            id, image_yaml.name
          )));
        }
      }
    } else if let image::BaseOrIms::Base { base } = &image_yaml.base_or_ims {
      if let image::Base::ImageRef { image_ref } = base {
        // New format
        // Validate base image
        log::debug!(
          "Validate 'image' '{image_name}' base image '{image_ref}'"
        );

        // Check there is another image with 'ref_name' that matches this 'image_ref'
        let image_found = image_yaml_vec.iter().any(|image_yaml| {
          image_yaml
            .ref_name
            .as_ref()
            .is_some_and(|ref_name| ref_name.eq(image_ref))
        });

        if !image_found {
          return Err(Error::SatFile(format!(
            "Could not find image with ref name '{}' in SAT file. Cancelling image build proccess. Exit",
            image_ref.as_str(),
          )));
        }
      // } else if let Some(image_base_product) = image_yaml["base"].get("product")
      } else if let image::Base::Product { product } = base {
        // Check if the 'Cray/HPE product' in CSM exists

        log::debug!("Image '{image_name}' base.base.product");
        log::debug!("SAT file - 'image.base.product' job");

        // Base image created from a cray product

        let product_name = &product.name;

        let product_version = product.version.as_ref().ok_or_else(|| {
          Error::YamlShape(format!(
            "SAT file: image '{image_name}' base.product '{product_name}' is missing 'version'"
          ))
        })?;

        let product_type = &product.r#type;

        let product_catalog_rslt = &serde_yaml::from_str::<serde_json::Value>(
          cray_product_catalog
            .get(product_name)
            .unwrap_or(&String::new()),
        );

        let Ok(product_catalog) = product_catalog_rslt else {
          return Err(Error::CrayProductCatalog(format!(
            "Product catalog for image '{image_name}' not found. Exit"
          )));
        };

        let product_type_opt = product_catalog
          .get(product_version)
          .and_then(|product_version| product_version.get(product_type.clone()))
          .cloned();

        let product_type_opt = if let Some(product_type) = product_type_opt {
          product_type.as_object().cloned()
        } else {
          return Err(Error::CrayProductCatalog(format!(
            "Product catalog for image '{image_name}' not found. Exit"
          )));
        };

        let image_map: Map<String, serde_json::Value> =
          if let Some(product_type) = &product_type_opt {
            product_type.clone()
          } else {
            return Err(Error::CrayProductCatalog(format!(
              "Product catalog for image '{image_name}' not found. Exit"
            )));
          };

        log::debug!(
          "CRAY product catalog items related to product name '{product_name}', product version '{product_version}' and product type '{product_type}':\n{product_type_opt:#?}"
        );

        if let Some(filter) = &product.filter {
          let image_recipe_id =
            filter_product_catalog_images(filter, image_map, image_name);
          image_recipe_id.is_ok()
        } else {
          // There is no 'image.product.filter' value defined in SAT file. Check Cray
          // product catalog only has 1 image. Othewise fail
          log::debug!(
            "No 'image.product.filter' defined in SAT file. Checking Cray product catalog only/must have 1 image"
          );
          image_map
            .values()
            .next()
            .is_some_and(|value| value.get("id").is_some())
        };
      // } else if let Some(image_base_ims_yaml) = image_yaml["base"].get("ims") {
      } else if let image::Base::Ims { ims } = base {
        // Check if the image exists

        log::debug!("Image '{image_name}' base.base.ims");
        if let image::ImageBaseIms::NameType { name, r#type } = ims {
          // if let Some(image_base_ims_name_yaml) = ims.get("name") {
          let image_base_ims_name_to_find = name;

          // Search image in SAT file

          log::debug!(
            "Searching base image '{image_base_ims_name_to_find}' related to image '{image_name}' in SAT file"
          );

          let mut image_found = image_yaml_vec
            .iter()
            .any(|image_yaml| image_yaml.name.eq(name));

          if !image_found {
            log::warn!(
              "Base image '{image_base_ims_name_to_find}' not found in SAT file, looking in CSM"
            );

            let image_base_ims_type = r#type;
            if image_base_ims_type.eq("recipe") {
              // Base IMS type is a recipe
              // Search in CSM (IMS Recipe)

              log::debug!(
                "Searching base image recipe '{image_base_ims_name_to_find}' related to image '{image_name}' in CSM"
              );

              image_found = ims_recipe_vec
                .iter()
                .any(|recipe| recipe.name.eq(image_base_ims_name_to_find));

              if !image_found {
                return Err(Error::SatFile(format!(
                  "Could not find IMS recipe '{image_base_ims_name_to_find}' in CSM. Cancelling image build proccess. Exit",
                )));
              }
            } else {
              // Base IMS type is an image
              // Search in CSM (IMS Image)

              log::debug!(
                "Searching base image '{image_base_ims_name_to_find}' related to image '{image_name}' in CSM"
              );

              // CFS session sets a custom image name, therefore we can't seach
              // for exact image name but search by substring
              image_found = image_vec
                .iter()
                .any(|image| image.name.contains(image_base_ims_name_to_find));

              if !image_found {
                return Err(Error::SatFile(format!(
                  "Could not find image base '{image_base_ims_name_to_find}' in image '{image_name}'. Cancelling image build proccess. Exit"
                )));
              }
            }
          }
        } else {
          log::warn!(
            "Image '{image_name}' is missing the field 'base.ims.name'. Exit"
          );
        }
      } else {
        return Err(Error::SatFile(format!(
          "Image '{image_name}' yaml not recognised. Exit"
        )));
      }
    } else {
      return Err(Error::SatFile(format!(
        "Image '{image_name}' neither have 'ims' nor 'base' value. Exit"
      )));
    }

    // Validate CFS configuration exists (image.configuration)
    log::debug!("Validate 'image' '{image_name}' configuration");

    if let Some(configuration_yaml) = image_yaml.configuration.as_ref() {
      let configuration_name_to_find = configuration_yaml;

      log::debug!(
        "Searching configuration name '{configuration_name_to_find}' related to image '{image_name}' in SAT file"
      );

      let mut configuration_found =
        configuration_yaml_vec.iter().any(|configuration_yaml| {
          configuration_yaml.name.eq(configuration_name_to_find)
        });

      if !configuration_found {
        // CFS configuration in image not found in SAT file, searching in CSM
        log::warn!(
          "Configuration '{configuration_name_to_find}' not found in SAT file, looking in CSM"
        );

        log::debug!(
          "Searching configuration name '{}' related to image '{}' in CSM",
          configuration_name_to_find,
          image_yaml.name
        );

        configuration_found = configuration_vec.iter().any(|configuration| {
          configuration.name.eq(configuration_name_to_find)
        });

        if !configuration_found {
          return Err(Error::SatFile(format!(
            "Could not find configuration '{configuration_name_to_find}' in image '{image_name}'. Cancelling image build proccess. Exit"
          )));
        }
      }

      // Validate user has access to HSM groups in 'image' section
      log::debug!("Validate 'image' '{image_name}' HSM groups");

      // Strip site-wide group names — see `hsm::group::hacks` module
      // docs for why.
      let configuration_group_names_vec =
        hsm::group::hacks::filter_system_hsm_group_names(
          image_yaml
            .configuration_group_names
            .clone()
            .unwrap_or_default(),
        );

      if configuration_group_names_vec.is_empty() {
        return Err(Error::SatFile(format!(
          "Image '{image_name}' must have group name values assigned to it. Canceling image build process. Exit"
        )));
      }
      for hsm_group in
        configuration_group_names_vec.iter().filter(|&hsm_group| {
          !hsm_group.eq_ignore_ascii_case("Compute")
            && !hsm_group.eq_ignore_ascii_case("Application")
            && !hsm_group.eq_ignore_ascii_case("Application_UAN")
        })
      {
        if !hsm_group_available_vec.contains(hsm_group) {
          return Err(Error::SatFile(format!(
            "HSM group '{}' in image '{}' not allowed, List of HSM groups available:\n{:?}. Exit",
            hsm_group, image_yaml.name, hsm_group_available_vec
          )));
        }
      }
    }
  }

  Ok(())
}
