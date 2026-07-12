//! Helpers built on top of `ShastaClient::ims_job_*` methods.

use crate::{ShastaClient, error::Error, ims::job::types::Job};

/// Wait for an IMS job to finish (polls every 2s, max 1800 attempts ~ 1h).
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn wait_ims_job_to_finish(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  ims_job_id: &str,
) -> Result<(), Error> {
  let client = ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?;
  let mut i = 0;
  let max = 1800;
  loop {
    let ims_job: Job = client
      .ims_job_get(shasta_token, Some(ims_job_id))
      .await?
      .first()
      .cloned()
      .ok_or_else(|| {
        Error::Message(format!("ERROR - IMS job '{ims_job_id}' not found"))
      })?;

    log::debug!(
      "IMS job details:\n{}",
      serde_json::to_string_pretty(&ims_job).unwrap_or_default()
    );

    let ims_job_status = ims_job.status.unwrap_or_default();

    if (ims_job_status != "error" && ims_job_status != "success") && i < max {
      log::debug!(
        "Waiting IMS job '{ims_job_id}' with job status '{ims_job_status}'. Checking again in 2 secs. Attempt {i} of {max}."
      );
      tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

      i += 1;
    } else {
      log::debug!(
        "\nIMS job '{ims_job_id}' finished with job status '{ims_job_status}'"
      );
      break;
    }
  }

  Ok(())
}
