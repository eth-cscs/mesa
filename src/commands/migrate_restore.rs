//! Restore a system from the bundle produced by [`crate::commands::migrate_backup`].

use crate::bos::BosSessionTemplate;
use crate::cfs::v3::{CfsConfigurationRequest, CfsConfigurationResponse};
use crate::hsm::group::types::Group;
use crate::ims;
use crate::ims::image::utils::{get_by_name, get_fuzzy};
use crate::ims::s3_client::BAR_FORMAT;
use crate::ims::{Image, Link};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use md5::Digest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

use crate::error::Error;
use crate::ims::PatchImage;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Artifact {
  pub link: Link,
  pub md5: String,
  #[serde(rename = "type")]
  pub r#type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ImageManifest {
  pub created: String,
  #[serde(default = "default_version")]
  pub version: String,
  pub artifacts: Vec<Artifact>,
}

fn default_version() -> String {
  "1.0".to_string()
}

/// Restore a system from the bundle produced by
/// [`crate::commands::migrate_backup::exec`].
///
/// Reads the BOS, CFS, HSM, and IMS JSON files plus the IMS image
/// directory and recreates the corresponding CSM resources on the
/// target system.
///
/// # Arguments
///
/// All four `*_file` arguments and `image_dir` are required (typed as
/// `Option` only because of the CLI surface that consumes this).
///
/// The four `overwrite_*` flags replace existing CSM resources with
/// the same name instead of failing.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn exec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  bos_file: Option<&str>,
  cfs_file: Option<&str>,
  hsm_file: Option<&str>,
  ims_file: Option<&str>,
  image_dir: Option<&str>,
  overwrite_group: bool,
  overwrite_configuration: bool,
  overwrite_image: bool,
  overwrite_template: bool,
) -> Result<(), Error> {
  fn require<'a>(opt: Option<&'a str>, name: &str) -> Result<&'a str, Error> {
    opt.ok_or_else(|| {
      Error::MigrateOp(format!("Error, --{name} argument is required."))
    })
  }

  let bos_file = require(bos_file, "bos-file")?;
  let cfs_file = require(cfs_file, "cfs-file")?;
  let hsm_file = require(hsm_file, "hsm-file")?;
  let ims_file = require(ims_file, "ims-file")?;
  let image_dir = require(image_dir, "image-dir")?;

  for (label, path) in [
    ("bos", bos_file),
    ("cfs", cfs_file),
    ("ims", ims_file),
    ("hsm", hsm_file),
  ] {
    if !PathBuf::from(path).exists() {
      return Err(Error::MigrateOp(format!(
        "Error, {label} file {path} does not exist or cannot be open."
      )));
    }
  }

  // ========================================================================================================
  let current_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
  let mut ims_image_manifest = ImageManifest {
    created: current_timestamp.to_string(),
    version: "1.0".to_string(),
    artifacts: vec![],
  };

  let backup_ims_file = ims_file.to_string();
  let backup_cfs_file = cfs_file.to_string();
  let backup_bos_file = bos_file.to_string();
  let backup_hsm_file = hsm_file.to_string();

  let ims_image_name: String = get_image_name_from_ims_file(&backup_ims_file)?;
  log::info!(" Image name: {ims_image_name}");

  let initrd_path = format!("{image_dir}/initrd");
  let kernel_path = format!("{image_dir}/kernel");
  let rootfs_path = format!("{image_dir}/rootfs");

  log::info!("\tinitrd file: {initrd_path}");
  log::info!("\tkernel file: {kernel_path}");
  log::info!("\trootfs file: {rootfs_path}");

  // These should come from the manifest, but let's assume these values are correct
  let vec_backup_image_files = vec![initrd_path, kernel_path, rootfs_path];

  for file in &vec_backup_image_files {
    if !PathBuf::from(&file).exists() {
      return Err(Error::MigrateOp(format!(
        "Error, file {} does not exist or cannot be open.",
        &file
      )));
    }
  }

  log::info!("Calculating image artifact checksum...");
  calculate_image_checksums(&mut ims_image_manifest, &vec_backup_image_files)?;

  // Do we have another image with this name?
  log::info!("\n\nRegistering image with IMS...");
  let ims_image_id_rslt = ims_register_image(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &ims_image_name,
    overwrite_image,
  )
  .await;

  let ims_image_id: String = match ims_image_id_rslt {
    Ok(value) => value,
    Err(e) => {
      return Err(Error::MigrateOp(format!("{e}")));
    }
  };

  log::info!("IMS image ID: {}", &ims_image_id);

  log::info!("\nUploading image artifacts to s3...");
  s3_upload_image_artifacts(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &ims_image_id,
    &mut ims_image_manifest,
    &vec_backup_image_files,
  )
  .await?;
  log::info!("\nUpdating IMS image record with the new location in s3...");
  log::debug!(
    "Updating image record with location of the newly generated manifest.json data"
  );
  ims_update_image_add_manifest(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &ims_image_name,
    &ims_image_id,
  )
  .await?;

  log::info!("\nCreating HSM group...");
  create_hsm_group_from_file(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &backup_hsm_file,
    overwrite_group,
  )
  .await?;

  log::info!("\nUploading CFS configuration...");
  // create a new CFS configuration based on the original CFS file backed up previously
  // this operation is simple as the file only has git repos and commits
  create_cfs_config(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &backup_cfs_file,
    overwrite_configuration,
  )
  .await?;

  log::info!("\nUploading BOS sessiontemplate...");

  // Create a new BOS session template based on the original BOS file backed previously
  create_bos_sessiontemplate(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &backup_bos_file,
    &ims_image_id,
    overwrite_template,
  )
  .await?;

  log::info!(
    "\nDone, the image bundle, HSM group, CFS configuration and BOS sessiontemplate have been restored."
  );

  // ========================================================================================================

  Ok(())
}

async fn create_bos_sessiontemplate(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  bos_file: &str,
  ims_image_id: &str,
  overwrite: bool,
) -> Result<(), Error> {
  let file_content = File::open(bos_file)?;

  let bos_json: BosSessionTemplate =
    serde_json::from_reader(BufReader::new(file_content))?;

  let bos_sessiontemplate_name = bos_json.name.ok_or_else(|| {
    Error::MigrateOp(
      "BOS sessiontemplate file is missing the 'name' field".to_string(),
    )
  })?;

  // BOS sessiontemplates need the new ID of the image!
  log::debug!("BOS sessiontemplate name: {}", &bos_sessiontemplate_name);

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let vector = shasta_client
    .bos_template_v2_get(shasta_token, Some(&bos_sessiontemplate_name))
    .await
    .map_err(|error| {
      Error::MigrateOp(format!(
        "Unable to query CSM to get list of BOS sessiontemplates: {error}"
      ))
    })?;

  log::debug!("BOS sessiontemplate filtered: {vector:#?}");

  if !vector.is_empty() {
    if overwrite {
      match shasta_client
        .bos_template_v2_delete(shasta_token, &bos_sessiontemplate_name)
        .await
      {
        Ok(()) => log::debug!(
          "Ok BOS session template {}, deleted.",
          &bos_sessiontemplate_name
        ),
        Result::Err(err1) => {
          return Err(Error::MigrateOp(format!(
            "unable to delete BOS session template: {err1}"
          )));
        }
      }
    } else {
      return Err(Error::MigrateOp(
        "BOS sessiontemplate already exists and --overwrite was not set".to_string(),
      ));
    }
  }

  let file_content = File::open(bos_file)?;

  let mut bos_sessiontemplate: BosSessionTemplate =
    serde_json::from_reader(BufReader::new(file_content))?;

  let path_modified =
    format!("s3://boot-images/{ims_image_id}/manifest.json");

  bos_sessiontemplate
    .boot_sets
    .as_mut()
    .and_then(|boot_sets| boot_sets.get_mut("compute"))
    .ok_or_else(|| {
      Error::MigrateOp(
        "BOS sessiontemplate has no 'compute' boot_set to update".to_string(),
      )
    })?
    .path = Some(path_modified);

  log::debug!("BOS sessiontemplate loaded:\n{bos_sessiontemplate:#?}");
  log::debug!("BOS sessiontemplate modified:\n{:#?}", &bos_sessiontemplate);

  match shasta_client
    .bos_template_v2_put(
      shasta_token,
      &bos_sessiontemplate,
      &bos_sessiontemplate_name,
    )
    .await
  {
    Ok(_result) => log::info!(
      "Ok, BOS session template {} created successfully.",
      &bos_sessiontemplate_name
    ),
    Err(e1) => {
      return Err(Error::MigrateOp(format!(
        "unable to create BOS session template: {e1}"
      )));
    }
  }

  Ok(())
}

/// Creates a CFS config on the current CSM system, based on the CFS file generated by manta migrate backup
/// panics with an error message if creation fails
async fn create_cfs_config(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  cfs_file: &str,
  overwrite: bool,
) -> Result<(), Error> {
  let file_content = File::open(cfs_file)?;

  let cfs_configuration: CfsConfigurationResponse =
    serde_json::from_reader(BufReader::new(file_content))?;

  // CFS needs to be cleaned up when loading into the system, the filed lastUpdate should not exist
  let cfs_config_name = cfs_configuration.name;

  // Get all CFS configurations, this is ugly
  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let cfs_config_vec = shasta_client
    .cfs_configuration_v3_get(shasta_token, Some(&cfs_config_name))
    .await
    .map_err(|error| {
      Error::MigrateOp(format!("Unable to fetch CFS configuration: {error}"))
    })?;

  if !cfs_config_vec.is_empty() {
    if !overwrite {
      return Err(Error::MigrateOp(
        "CFS configuration already exists and --overwrite was not set".to_string(),
      ));
    }

    match shasta_client
      .cfs_configuration_v3_delete(shasta_token, cfs_config_name.as_str())
      .await
    {
      Ok(()) => {
        log::debug!("Ok CFS configuration {cfs_config_name}, deleted.");
      }
      Result::Err(error) => {
        return Err(Error::MigrateOp(format!(
          "unable to delete CFS configuration {cfs_config_name}: {error}"
        )));
      }
    }
  }

  // At this point we're sure there's either no CFS config with that name
  // or that the user wants to overwrite it, so let's do it

  let file_content = File::open(cfs_file)?;

  let cfs_configuration: CfsConfigurationRequest =
    serde_json::from_reader(BufReader::new(file_content))?;

  log::debug!("CFS config:\n{:#?}", &cfs_configuration);

  match shasta_client
    .cfs_configuration_v3_put(
      shasta_token,
      &cfs_configuration,
      cfs_config_name.as_str(),
    )
    .await
  {
    Ok(result) => {
      log::debug!("Ok, result: {result:#?}");
      log::info!(
        "Ok, CFS configuration {} created successfully.",
        &cfs_config_name
      );
    }
    Err(e1) => {
      return Err(Error::MigrateOp(format!(
        "unable to create CFS configuration: {e1}"
      )));
    }
  }

  Ok(())
}

/// Add the image manifest field to an IMS image record
/// the manifest field will be: <s3://boot-images/{ims_image_id}/manifest.json>
async fn ims_update_image_add_manifest(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  ims_image_name: &str,
  ims_image_id: &str,
) -> Result<(), Error> {
  match get_fuzzy(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &[String::new()], // hsm_group_name
    Some(ims_image_name),
    None,
  )
  .await
  {
    Ok(vector) => {
      if vector.is_empty() {
        return Err(Error::MigrateOp(format!(
          "no images stored with id {} in IMS, unable to update the image manifest",
          &ims_image_id
        )));
      }
    }
    Err(error) => {
      return Err(Error::MigrateOp(format!(
        "unable to determine if there are other images in IMS with the name {}: {}",
        &ims_image_name, &error
      )));
    }
  }

  let _ims_record = ims::image::http_client::types::Image {
    name: ims_image_name.to_string(),
    id: Some(ims_image_id.to_string()),
    created: None,
    arch: None,
    link: Some(ims::image::http_client::types::Link {
      etag: None,
      path: format!(
        "s3://boot-images/{}/manifest.json",
        &ims_image_id.to_string()
      ),
      r#type: "s3".to_string(),
    }),
    metadata: None,
  };

  let ims_link = Link {
    etag: None,
    path: format!(
      "s3://boot-images/{}/manifest.json",
      &ims_image_id.to_string()
    ),
    r#type: "s3".to_string(),
  };
  let rec = PatchImage {
    link: Some(ims_link),
    arch: None,
    metadata: None,
  };

  let patch_result = match crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  ) {
    Ok(client) => {
      client
        .ims_image_patch(shasta_token, ims_image_id, &rec)
        .await
    }
    Err(e) => Err(e),
  };

  match patch_result {
    Ok(()) => log::debug!("Image updated"),
    Err(e) => {
      return Err(Error::MigrateOp(format!(
        "unable to modify the record of the image: {e}"
      )));
    }
  }

  Ok(())
}

/// Uploads to s3 under boot-images/ims_image_id all the files that
/// `vec_image_files` refers to. If upload successful, it modifies
/// `ImageManifest` to point to the right place within s3
async fn s3_upload_image_artifacts(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  ims_image_id: &str,
  ims_image_manifest: &mut ImageManifest,
  vec_image_files: &Vec<String>,
) -> Result<(), Error> {
  let bucket_name = "boot-images";
  let object_path = ims_image_id;

  // Connect and auth to S3
  let sts_value = match ims::s3_client::s3_auth(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
  )
  .await
  {
    Ok(sts_value) => sts_value,
    Err(error) => {
      return Err(Error::MigrateOp(format!(
        "unable to authenticate with s3 when uploading images: {error}"
      )));
    }
  };

  for file in vec_image_files {
    let filename = Path::new(file).file_name().ok_or_else(|| {
      Error::MigrateOp(format!("Path '{file}' has no file name component"))
    })?;
    let file_size = match fs::metadata(file) {
      Ok(file_metadata) => crate::fmt::format_bytes(file_metadata.len()),
      Err(e) => {
        log::warn!(
          "Unable to fetch file metadata info, faking the value. Error: {e}"
        );
        "-1".to_string()
      }
    };

    let full_object_path =
      format!("{}/{}", &object_path, &filename.to_string_lossy());
    log::info!(
      "File {:?} ({}) to s3://{}/{}.",
      &file,
      &file_size,
      &bucket_name,
      &full_object_path
    );
    let etag: String = if fs::metadata(file)?.len() > 1024 * 1024 * 5 {
      match ims::s3_client::s3_multipart_upload_object(
        &sts_value,
        &full_object_path,
        bucket_name,
        file,
      )
      .await
      {
        Ok(result) => {
          log::debug!("Artifact uploaded successfully.");
          result
        }
        Err(error) => {
          return Err(Error::MigrateOp(format!(
            "unable to upload file to s3: {error}"
          )));
        }
      }
    } else {
      match ims::s3_client::s3_upload_object(
        &sts_value,
        &full_object_path,
        bucket_name,
        file,
      )
      .await
      {
        Ok(result) => {
          log::info!("Ok");
          result
        }
        Err(error) => {
          return Err(Error::MigrateOp(format!(
            "unable to upload file to s3: {error}"
          )));
        }
      }
    };

    // I'm pretty sure there's a better way to do this...
    if file.contains("kernel") {
      for artifact in &mut ims_image_manifest.artifacts {
        if !etag.is_empty() {
          // assign eTag if returned by s3, otherwise set to none
          artifact.link.etag = Some(etag.clone());
        }
        if artifact.r#type.contains("kernel") {
          artifact.link.path = "s3://".to_string()
            + bucket_name
            + "/"
            + object_path
            + "/kernel";
          break;
        }
      }
    } else if file.contains("rootfs") {
      for artifact in &mut ims_image_manifest.artifacts {
        if !etag.is_empty() {
          // assign eTag if returned by s3, otherwise set to none
          artifact.link.etag = Some(etag.clone());
        }
        if artifact.r#type.contains("rootfs") {
          artifact.link.path = "s3://".to_string()
            + bucket_name
            + "/"
            + object_path
            + "/rootfs";
          break;
        }
      }
    } else if file.contains("initrd") {
      for artifact in &mut ims_image_manifest.artifacts {
        if !etag.is_empty() {
          // assign eTag if returned by s3, otherwise set to none
          artifact.link.etag = Some(etag.clone());
        }
        if artifact.r#type.contains("initrd") {
          artifact.link.path = "s3://".to_string()
            + bucket_name
            + "/"
            + object_path
            + "/initrd";
          break;
        }
      }
    }
  }

  log::debug!("Writing the new manifest.json file with the correct new ID");

  let new_manifest_file_name = String::from("new-manifest.json");

  let first_image_file = vec_image_files.first().ok_or_else(|| {
    Error::MigrateOp(
      "vec_image_files is empty; no manifest can be written".to_string(),
    )
  })?;
  let new_manifest_file_path = Path::new(first_image_file)
    .parent()
    .map(|path| path.join(&new_manifest_file_name))
    .ok_or_else(|| {
      Error::MigrateOp(format!(
        "Path '{first_image_file}' has no parent directory"
      ))
    })?;

  let new_manifest_file = File::create(&new_manifest_file_path)?;

  serde_json::to_writer_pretty(&new_manifest_file, &ims_image_manifest)?;

  log::debug!("Uploading the new manifest.json file");
  let manifest_full_object_path = format!("{}/manifest.json", &object_path);
  log::info!(
    "File {:?} -> s3://{}/{}.",
    &new_manifest_file_name,
    &bucket_name,
    &manifest_full_object_path
  );

  match ims::s3_client::s3_upload_object(
    &sts_value,
    &manifest_full_object_path,
    bucket_name,
    &new_manifest_file_path.clone().to_string_lossy(),
  )
  .await
  {
    Ok(_result) => {
      log::info!("OK");
    }
    Err(error) => {
      return Err(Error::MigrateOp(format!(
        "unable to upload file to s3: {error}"
      )));
    }
  }

  Ok(())
}

/// Return the md5sum of a file. Returns `Err` if the file cannot be
/// opened, its metadata cannot be read, or an I/O error occurs while
/// reading.
fn file_md5sum(filename: PathBuf) -> Result<Digest, Error> {
  log::debug!("File {}...", filename.display());

  let f = File::open(&filename).map_err(|e| {
    Error::MigrateOp(format!("file_md5sum: open {}: {e}", filename.display()))
  })?;
  // Find the length of the file
  let len = f
    .metadata()
    .map_err(|e| {
      Error::MigrateOp(format!(
        "file_md5sum: metadata {}: {e}",
        filename.display()
      ))
    })?
    .len();
  // Decide on a reasonable buffer size (100MB in this case, fastest will depend on hardware)
  let buf_len = len.min(100_000_000) as usize;
  let mut buf = BufReader::with_capacity(buf_len, f);
  let mut context = md5::Context::new();
  let bar = ProgressBar::new(len);
  // BAR_FORMAT is a compile-time constant — template parse is infallible.
  bar.set_style(
    ProgressStyle::with_template(BAR_FORMAT).expect("BAR_FORMAT is valid"),
  );

  loop {
    // Get a chunk of the file
    let part = buf.fill_buf().map_err(|e| {
      Error::MigrateOp(format!(
        "file_md5sum: read {}: {e}",
        filename.display()
      ))
    })?;
    // If that chunk was empty, the reader has reached EOF
    if part.is_empty() {
      break;
    }
    // Add chunk to the md5
    context.consume(part);
    // Tell the buffer that the chunk is consumed
    let part_len = part.len();
    buf.consume(part_len);
    bar.inc(part_len as u64);
  }
  let digest = context.compute();
  bar.finish();

  Ok(digest)
}
/// Calculates the md5sum of all the files in the `vec_backup_image_files` vector and updates
///  the image manifest at `ims_image_manifest`
fn calculate_image_checksums(
  image_manifest: &mut ImageManifest,
  vec_backup_image_files: &Vec<String>,
) -> Result<(), Error> {
  for file in vec_backup_image_files {
    let file_size = match fs::metadata(file) {
      Ok(file_metadata) => crate::fmt::format_bytes(file_metadata.len()),
      Err(e) => {
        log::warn!(
          "Unable to fetch file metadata info, faking the value. Error: {e}"
        );
        "-1".to_string()
      }
    };
    log::info!("File {:?} ({})...", &file, &file_size);
    let artifact;
    let mut fp = PathBuf::new();
    fp.push(file);
    let digest = file_md5sum(fp)?;

    if file.contains("kernel") {
      artifact = Artifact {
        md5: format!("{digest:x}"),
        link: Link {
          path: "path".to_string(),
          r#type: "s3".to_string(),
          etag: None,
        },
        r#type: "application/vnd.cray.image.kernel".to_string(),
      };
    } else if file.contains("rootfs") {
      artifact = Artifact {
        md5: format!("{digest:x}"),
        link: Link {
          path: "path".to_string(),
          r#type: "s3".to_string(),
          etag: None,
        },
        r#type: "application/vnd.cray.image.rootfs.squashfs".to_string(),
      };
    } else {
      artifact = Artifact {
        md5: format!("{digest:x}"),
        link: Link {
          path: "path".to_string(),
          r#type: "s3".to_string(),
          etag: None,
        },
        r#type: "application/vnd.cray.image.initrd".to_string(),
      };
    }
    image_manifest.artifacts.push(artifact);
  }
  Ok(())
}

/// Registers in IMS a new image and returns the new id to pass to s3
async fn ims_register_image(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  ims_image_name: &str,
  overwrite: bool,
) -> Result<String, Error> {
  let ims_record = Image {
    name: ims_image_name.to_string(),
    id: None,
    created: None,
    link: None,
    arch: None,
    metadata: None,
  };

  let list_images_with_same_name = get_by_name(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &[String::new()], // hsm_group_name
    ims_image_name,
    None,
  )
  .await?;

  if !list_images_with_same_name.is_empty() && !overwrite {
    return Err(Error::MigrateOp(format!(
      "IMS image '{ims_image_name}' already exists and --overwrite-image was not set"
    )));
  }

  let json_response = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .ims_image_post(shasta_token, &ims_record)
  .await?;

  json_response
    .get("id")
    .and_then(Value::as_str)
    .map(|id| id.replace('"', ""))
    .ok_or_else(|| {
      Error::MigrateOp(
        "IMS image post response is missing 'id'".to_string(),
      )
    })
}

/// Gets the image name off an IMS yaml file
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub fn get_image_name_from_ims_file(
  ims_file: &str,
) -> Result<String, Error> {
  // load into memory
  let ims_data = fs::read_to_string(PathBuf::from(&ims_file))?;

  let ims_json: serde_json::Value = serde_json::from_str(&ims_data)?;

  ims_json
    .pointer("/0/name")
    .and_then(Value::as_str)
    .map(|ims_name| ims_name.replace('"', ""))
    .ok_or_else(|| {
      Error::MigrateOp(format!("IMS file '{ims_file}' is missing /0/name"))
    })
}

/// Recreate HSM groups from a backup file produced by
/// [`crate::commands::migrate_backup::exec`].
///
/// `hsm_file` is the path to a JSON list of `Group` objects, e.g.
/// `[{"gele":["x1001c7s1b1n1", …]}]`.
///
/// Failures inside this function are treated as fatal — the migrate
/// flow cannot continue if HSM group creation is partial.
// Anything in this function is critical, so the asserts will kill further processing
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn create_hsm_group_from_file(
  // backend: &StaticBackendDispatcher,
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  hsm_file: &str,
  overwrite: bool,
) -> Result<(), Error> {
  // Parse HSM group file
  // The file looks like this: [{"gele":["x1001c7s1b1n1","x1001c7s1b0n0","x1001c7s1b1n0","x1001c7s1b0n1"]}]
  // load into memory
  let hsm_data = fs::read_to_string(PathBuf::from(hsm_file))?;

  let group_vec: Vec<Group> = serde_json::from_str(&hsm_data)?;

  let shasta_client = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  for group in group_vec {
    // Create the HSM group.
    //
    // Post-progenitor shape gymnastics:
    // - `Group.members.ids` is `Vec<XNameRw100>` (no `Option`).
    //   Map out the newtype to feed `hsm_group_create_new_group`'s
    //   `&[String]` parameter.
    // - `Group.label`/`exclusive_group` are `ResourceName(pub String)`;
    //   reach into `.0` for the `&str` arg.
    // - `Group.tags` is `Vec<ResourceName>` (no `Option`); same
    //   unwrap-the-newtype dance for the `&[String]` parameter.
    let group_members_vec: Vec<String> = group
      .members
      .as_ref()
      .map(|m| m.ids.iter().map(|x| x.0.clone()).collect())
      .unwrap_or_default();
    let tags_vec: Vec<String> =
      group.tags.iter().map(|t| t.0.clone()).collect();
    let exclusive_group_str = group
      .exclusive_group
      .as_ref()
      .map(|x| x.0.as_str())
      .unwrap_or_default();
    match shasta_client
      .hsm_group_create_new_group(
        shasta_token,
        &group.label.0,
        &group_members_vec,
        exclusive_group_str,
        group.description.as_deref().unwrap_or_default(),
        &tags_vec,
      )
      .await
    {
      Ok(group) => {
        log::info!(
          "The HSM group {} has been created successfully.",
          &group.label.0
        );
      }
      Err(error) => {
        if error.to_string().to_lowercase().contains("409") {
          if overwrite {
            log::info!("Looks like you want to continue");
            match shasta_client
              .hsm_group_delete_group(shasta_token, &group.label.0)
              .await
            {
              Ok(_) => {
                // try creating the group again
                match shasta_client
                  .hsm_group_post(shasta_token, group.clone())
                  .await
                {
                  Ok(_json) => {
                    log::info!(
                      "The HSM group {} has been created successfully.",
                      &group.label.0
                    );
                  }
                  Err(e) => {
                    log::error!("Error message {e}");
                    return Err(Error::MigrateOp(format!(
                      "second error creating a new HSM group: {e}"
                    )));
                  }
                }
              }
              Err(e) => {
                log::error!("Error message {e}");
                return Err(Error::MigrateOp(format!(
                  "error deleting the HSM group {}: {}",
                  &group.label.0, e
                )));
              }
            }
          } else {
            return Err(Error::MigrateOp(
              "Not deleting the group, cannot continue the operation.".to_string(),
            ));
          }
        } else if error.to_string().to_lowercase().contains("400") {
          return Err(Error::MigrateOp(
            "Unable to create the group, the API returned code 400. This usually means the HSM file is malformed, or has incorrect xnames for this site in it.".to_string(),
          ));
        }
      }
    }
  }

  Ok(())
}
