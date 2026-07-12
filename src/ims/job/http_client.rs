//! `ShastaClient` methods for `/ims/v3/jobs`.

use serde_json::Value;

use crate::{ShastaClient, common::http, error::Error};

use super::{
  types::{Job, SshContainer},
  utils::wait_ims_job_to_finish,
};

impl ShastaClient {
  /// Creates an IMS job of type 'customize'. Used to create
  /// 'ephemeral-environments'.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn ims_job_post_customize(
    &self,
    token: &str,
    image_root_archive_name: &str,
    artifact_id: &str,
    public_key_id: &str,
  ) -> Result<Value, Error> {
    let ssh_container_list = vec![SshContainer {
      name: "jail".to_string(),
      jail: true,
    }];

    let ims_job = Job {
      job_type: "customize".to_string(),
      image_root_archive_name: image_root_archive_name.to_string(),
      kernel_file_name: Some("kernel".to_string()),
      initrd_file_name: Some("initrd".to_string()),
      kernel_parameters_file_name: None,
      artifact_id: artifact_id.to_string(),
      public_key_id: public_key_id.to_string(),
      ssh_containers: Some(ssh_container_list),
      enable_debug: Some(false),
      build_env_size: None,
      require_dkms: None, // FIXME: check if SAT file uses this value
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

    let url = format!("{}/ims/v3/jobs", self.base_url());
    http::post_json(self.http(), &url, token, &ims_job).await
  }

  /// Creates an IMS job. Returns immediately after the create call.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn ims_job_post(
    &self,
    token: &str,
    ims_job: &Job,
  ) -> Result<Job, Error> {
    let api_url = format!("{}/ims/v3/jobs", self.base_url());

    self
      .http()
      .post(api_url)
      .bearer_auth(token)
      .json(&ims_job)
      .send()
      .await
      .map_err(Error::NetError)?
      .error_for_status()
      .map_err(Error::NetError)?
      .json()
      .await
      .map_err(Error::NetError)
  }

  /// Like `ims_job_post`, but waits for the job to finish before returning.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn ims_job_post_sync(
    &self,
    token: &str,
    ims_job: &Job,
  ) -> Result<Job, Error> {
    log::debug!("Create IMS job");
    log::debug!(
      "Create IMS job request payload:\n{}",
      serde_json::to_string_pretty(&ims_job)?
    );

    let created = self.ims_job_post(token, ims_job).await?;

    let ims_job_id = created.id.clone().ok_or_else(|| {
      Error::Message("IMS job creation response is missing 'id'".to_string())
    })?;

    // Wait till the IMS job finishes
    wait_ims_job_to_finish(
      token,
      self.base_url(),
      self.root_cert(),
      &ims_job_id,
    )
    .await?;

    self
      .ims_job_get(token, Some(&ims_job_id))
      .await?
      .first()
      .cloned()
      .ok_or_else(|| {
        Error::Message(format!("ERROR - IMS job '{ims_job_id}' not found"))
      })
  }

  /// `GET /ims/v3/jobs` (or `/ims/v3/jobs/{id}` if `job_id_opt` is
  /// supplied) — list IMS jobs or fetch one by ID.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn ims_job_get(
    &self,
    token: &str,
    job_id_opt: Option<&str>,
  ) -> Result<Vec<Job>, Error> {
    let api_url = if let Some(job_id) = job_id_opt {
      format!("{}/ims/v3/jobs/{}", self.base_url(), job_id)
    } else {
      format!("{}/ims/v3/jobs", self.base_url())
    };

    let response = self
      .http()
      .get(api_url)
      .bearer_auth(token)
      .send()
      .await
      .map_err(Error::NetError)?
      .error_for_status()
      .map_err(Error::NetError)?;

    if job_id_opt.is_some() {
      Ok(vec![response.json::<Job>().await.map_err(Error::NetError)?])
    } else {
      response.json().await.map_err(Error::NetError)
    }
  }
}
