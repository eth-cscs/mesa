//! Back up a BOS session template and its dependencies (CFS/HSM/IMS) to disk.

use crate::commands::migrate_restore;
use crate::error::Error;
use crate::{bos, ims};
use std::fs::File;
use std::path::Path;

// use crate::commands::i_migrate_restore;

/// Back up one BOS session template — plus the IMS image artefacts and
/// CFS/HSM metadata it references — to a local directory.
///
/// Produces a self-contained on-disk snapshot that
/// [`crate::commands::migrate_restore::exec`] can re-import on another
/// system.
///
/// # Arguments
///
/// - `bos` — name of the BOS session template to back up. Required.
/// - `destination` — directory to write the archive into; created if
///   missing. Required.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn exec(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  bos: Option<&str>,
  destination: Option<&str>,
  /* prehook: Option<&String>,
  posthook: Option<&String>, */
) -> Result<(), Error> {
  let bos = bos.ok_or_else(|| {
    Error::MigrateOp("Error, --bos argument is required.".to_string())
  })?;
  let destination = destination.ok_or_else(|| {
    Error::MigrateOp("Error, --destination argument is required.".to_string())
  })?;

  let dest_path = Path::new(destination);
  let bucket_name = "boot-images";
  let files2download = ["manifest.json", "initrd", "kernel", "rootfs"];
  let files2download_count = files2download.len() + 4; // manifest.json, initrd, kernel, rootfs, bos, cfs, hsm, ims
  log::debug!("Create directory '{destination}'");
  std::fs::create_dir_all(dest_path).map_err(|e| {
    Error::MigrateOp(format!(
      "Unable to create directory {}: {}",
      dest_path.to_string_lossy(),
      e
    ))
  })?;
  let bos_file_name = String::from(bos) + ".json";
  let bos_file_path = dest_path.join(bos_file_name);

  let hsm_file_name = String::from(bos) + "-hsm.json";
  let hsm_file_path = dest_path.join(hsm_file_name);

  let _empty_hsm_group_name: Vec<String> = Vec::new();
  let mut bos_templates = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .bos_template_v2_get(shasta_token, Some(bos))
  .await?;

  let _ =
    bos::template::utils::filter(&mut bos_templates, None, &[], &[], None);

  let mut download_counter = 1;

  if bos_templates.is_empty() {
    return Err(Error::MigrateOp("No BOS template found!".to_string()));
  } else {
    // BOS ------------------------------------------------------------------------------------
    let bos_file = File::create(&bos_file_path)?;

    log::info!(
      "Downloading BOS session template {} to {} [{}/{}]",
      bos,
      &bos_file_path.clone().to_string_lossy(),
      &download_counter,
      &files2download_count
    );

    // Save to file only the first one returned, we don't expect other BOS templates in the array
    let _bosjson = serde_json::to_writer_pretty(&bos_file, &bos_templates[0]);
    download_counter += 1;

    // HSM group -----------------------------------------------------------------------------

    let hsm_file = File::create(&hsm_file_path)?;
    log::info!(
      "Downloading HSM configuration in bos template {} to {} [{}/{}]",
      bos,
      &hsm_file_path.clone().to_string_lossy(),
      &download_counter,
      &files2download_count
    );
    download_counter += 1;

    let hsm_group_name = bos_templates
      .first()
      .and_then(|first_bos_template| first_bos_template.boot_sets.as_ref())
      .and_then(|boot_sets| boot_sets.get("compute"))
      .and_then(|compute_boot_set| compute_boot_set.node_groups.as_ref())
      .and_then(|node_groups| node_groups.first())
      .map(|node_group| node_group.replace('\"', ""))
      .ok_or_else(|| {
        Error::MigrateOp(format!(
          "BOS template '{bos}': no 'compute' boot_set or no node_groups"
        ))
      })?;

    let hsm_group_json = crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .hsm_group_get(shasta_token, Some(std::slice::from_ref(&hsm_group_name)), None)
    .await?;

    log::debug!("{:#?}", &hsm_group_json);
    let _hsmjson = serde_json::to_writer_pretty(&hsm_file, &hsm_group_json);

    // CFS ------------------------------------------------------------------------------------
    let configuration_name: &String = bos_templates
      .first()
      .and_then(|first_bos_template| first_bos_template.cfs.as_ref())
      .and_then(|cfs_value| cfs_value.configuration.as_ref())
      .ok_or_else(|| {
        Error::MigrateOp(format!(
          "BOS template '{bos}': no CFS configuration referenced"
        ))
      })?;

    let cfs_configurations = crate::ShastaClient::new(
      shasta_base_url,
      shasta_root_cert.to_vec(),
    )?
    .cfs_configuration_v3_get(shasta_token, Some(configuration_name))
    .await?;

    let cfs_file_name =
      String::from(configuration_name.clone().as_str()) + ".json";
    let cfs_file_path = dest_path.join(&cfs_file_name);
    let cfs_file = File::create(&cfs_file_path)?;

    log::info!(
      "Downloading CFS configuration {} to {} [{}/{}]",
      &configuration_name,
      &cfs_file_path.clone().to_string_lossy(),
      &download_counter,
      &files2download_count
    );

    // Save to file only the first one returned, we don't expect other BOS templates in the array
    let _cfsjson =
      serde_json::to_writer_pretty(&cfs_file, &cfs_configurations[0]);

    download_counter += 1;

    // Image ----------------------------------------------------------------------------------
    let boot_sets = bos_templates
      .first()
      .and_then(|first_bos_template| first_bos_template.boot_sets.as_ref())
      .ok_or_else(|| {
        Error::MigrateOp(format!("BOS template '{bos}': no boot_sets defined"))
      })?;
    for boot_sets_value in boot_sets.values() {
      if let Some(path) = &boot_sets_value.path {
        let image_id_related_to_bos_sessiontemplate = path
          .trim_start_matches("s3://boot-images/")
          .trim_end_matches("/manifest.json")
          .to_string();

        log::info!(
          "Get image details for ID {image_id_related_to_bos_sessiontemplate}"
        );
        let ims_file_name = String::from(
          image_id_related_to_bos_sessiontemplate.clone().as_str(),
        ) + "-ims.json";

        let ims_file_path = dest_path.join(&ims_file_name);
        let ims_file = File::create(&ims_file_path)?;

        log::info!(
          "Downloading IMS image record {} to {} [{}/{}]",
          &image_id_related_to_bos_sessiontemplate,
          &ims_file_path.clone().to_string_lossy(),
          &download_counter,
          &files2download_count
        );
        match crate::ShastaClient::new(
          shasta_base_url,
          shasta_root_cert.to_vec(),
        )?
        .ims_image_get(
          shasta_token,
          Some(&image_id_related_to_bos_sessiontemplate),
        )
        .await
        {
          Ok(ims_record) => {
            serde_json::to_writer_pretty(&ims_file, &ims_record)?;
            let image_id =
              image_id_related_to_bos_sessiontemplate.clone().clone();
            log::info!(
              "Image ID found related to BOS sessiontemplate {bos} is {image_id_related_to_bos_sessiontemplate}"
            );
            let sts_value = match ims::s3_client::s3_auth(
              shasta_token,
              shasta_base_url,
              shasta_root_cert,
            )
            .await
            {
              Ok(sts_value) => sts_value,

              Err(error) => return Err(Error::MigrateOp(error.to_string())),
            };
            for file in files2download {
              let dest = String::from(destination) + "/" + &image_id;
              let src = image_id.clone() + "/" + file;
              let object_size = ims::s3_client::s3_get_object_size(
                &sts_value,
                &src,
                bucket_name,
              )
              .await
              .unwrap_or(-1);
              log::info!(
                "Downloading image file {} ({}) to {}/{} [{}/{}]",
                &src,
                crate::fmt::format_bytes(object_size as u64),
                &dest,
                &file,
                &download_counter,
                &files2download_count
              );
              match ims::s3_client::s3_download_object(
                &sts_value,
                &src,
                bucket_name,
                &dest,
              )
              .await
              {
                Ok(_result) => {
                  download_counter += 1;
                }
                Err(error) => {
                  return Err(Error::MigrateOp(format!(
                    "unable to download file {} from s3. Error returned: {}",
                    &src, error
                  )));
                }
              }
            } // for file in files2download
            log::info!("\nDone, the following image bundle was generated:");
            log::info!("\tBOS file: {}", &bos_file_path.to_string_lossy());
            log::info!("\tCFS file: {}", &cfs_file_path.to_string_lossy());
            log::info!("\tHSM file: {}", &hsm_file_path.to_string_lossy());
            log::info!("\tIMS file: {}", &ims_file_path.to_string_lossy());
            let ims_image_name = migrate_restore::get_image_name_from_ims_file(
              &ims_file_path.to_string_lossy(),
            )?;
            log::info!("\tImage name: {ims_image_name}");
            for file in files2download {
              let dest = String::from(destination);
              let src = image_id.clone() + "/" + file;
              log::info!("\t\tfile: {dest}/{src}");
            }
          }
          Err(e) => {
            return Err(Error::MigrateOp(format!(
              "image related to BOS session template {image_id_related_to_bos_sessiontemplate} not found: {e}"
            )));
          }
        }
      }
    }
  }

  Ok(())
}
