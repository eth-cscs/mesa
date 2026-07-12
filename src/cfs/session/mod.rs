//! CFS sessions — invocations that run an Ansible configuration against
//! a set of components.
//!
//! Submodules:
//!
//! - [`http_client`] — `ShastaClient` methods for the v2 and v3 endpoints.
//! - [`utils`] — orchestration helpers that compose multiple calls.

pub mod http_client;
pub mod utils;

use http_client::v2::types::{CfsSessionGetResponse, CfsSessionPostRequest};

use crate::error::Error;

#[cfg(feature = "k8s-console")]
use crate::common::{
  kubernetes::{self, i_print_cfs_session_logs},
  vault::http_client::fetch_shasta_k8s_secrets_from_vault,
};

/// Fetch a single CFS session by name (errors if none or many match).
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_one(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  session_name: &String,
) -> Result<CfsSessionGetResponse, Error> {
  let cfs_session_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .cfs_session_v2_get(shasta_token, None, None, None, Some(session_name), None)
  .await?;

  let mut iter = cfs_session_vec.into_iter();
  match (iter.next(), iter.next()) {
    (Some(session), None) => Ok(session),
    _ => Err(Error::SessionNotFound(session_name.clone())),
  }
}

/// Fetch CFS sessions. Ref: <https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/>.
///
/// Returns list of CFS sessions ordered by start time. Filters by either
/// HSM group name or HSM group members or both.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[allow(clippy::too_many_arguments)]
pub async fn get_and_sort(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  min_age_opt: Option<&String>,
  max_age_opt: Option<&String>,
  status_opt: Option<&String>,
  session_name_opt: Option<&String>,
  is_succeded_opt: Option<bool>,
) -> Result<Vec<CfsSessionGetResponse>, Error> {
  let mut cfs_session_vec = crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .cfs_session_v2_get(
    shasta_token,
    min_age_opt,
    max_age_opt,
    status_opt,
    session_name_opt,
    is_succeded_opt,
  )
  .await?;

  // Sort CFS sessions by start time order ASC
  cfs_session_vec.sort_by_key(http_client::v2::types::CfsSessionGetResponse::get_start_time);

  Ok(cfs_session_vec)
}

/// Convenience: build a transient `ShastaClient`, POST the CFS session
/// request, and return the created session.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn post(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  session: &CfsSessionPostRequest,
) -> Result<CfsSessionGetResponse, Error> {
  log::info!("Create CFS session '{}'", session.name);
  log::debug!("Create CFS session request payload:\n{session:#?}");

  crate::ShastaClient::new(
    shasta_base_url,
    shasta_root_cert.to_vec(),
  )?
  .cfs_session_v2_post(shasta_token, session)
  .await
}

/// Creates a CFS session and waits for it to finish. When `watch_logs`
/// is true the session's container logs are streamed line-by-line
/// through `log::info!` (no direct stdout writes).
///
/// Requires the `k8s-console` Cargo feature because the watch-logs
/// path attaches to the in-cluster CFS session pod via the Kubernetes
/// client.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
#[cfg(feature = "k8s-console")]
#[allow(clippy::too_many_arguments)]
pub async fn i_post_sync(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
  vault_base_url: &str,
  site_name: &str,
  k8s_api_url: &str,
  session: &CfsSessionPostRequest,
  watch_logs: bool,
  timestamps: bool,
) -> Result<CfsSessionGetResponse, Error> {
  // Create CFS session
  log::info!("Create CFS session '{}'", session.name);
  let cfs_session: CfsSessionGetResponse = crate::cfs::session::post(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    session,
  )
  .await?;

  let cfs_session_name: String = cfs_session.name;

  // NOTE: the watch-logs block below mirrors the downstream
  // `manta apply sat-file` and `manta logs` flows; if you change the
  // shape here check that those still match (they live in the manta
  // CLI repo, not in csm-rs).
  if watch_logs {
    log::info!("Fetching logs form CFS session {} ...", session.name);
    let shasta_k8s_secrets = fetch_shasta_k8s_secrets_from_vault(
      vault_base_url,
      shasta_token,
      site_name,
    )
    .await?;

    let client =
      kubernetes::get_client(k8s_api_url, shasta_k8s_secrets).await?;

    i_print_cfs_session_logs(client, &cfs_session_name, timestamps).await?;
  }

  // User does not want the CFS logs but we still need to wait for the CFS session to
  // finish. Wait till the CFS session finishes
  utils::wait_cfs_session_to_finish(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &cfs_session_name,
  )
  .await?;

  // Get most recent CFS session status
  let cfs_session: CfsSessionGetResponse = get_one(
    shasta_token,
    shasta_base_url,
    shasta_root_cert,
    &cfs_session_name,
  )
  .await?;

  Ok(cfs_session)
}
