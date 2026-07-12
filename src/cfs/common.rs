//! Helpers shared across the CFS resource modules (e.g. service health).

use serde_json::Value;

use crate::{ShastaClient, common::http, error::Error};

impl ShastaClient {
  /// Probe CFS for liveness.
  ///
  /// `GET /cfs/healthz`. Returns the raw JSON body produced by CFS;
  /// success only means the service was reachable and authenticated.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn cfs_health_check(&self, token: &str) -> Result<Value, Error> {
    let api_url = format!("{}/cfs/healthz", self.base_url());
    http::get_json(self.http(), &api_url, token).await
  }
}

/// Convenience: build a transient `ShastaClient` and run a CFS health
/// check in one call.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn health_check(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
) -> Result<Value, Error> {
  crate::ShastaClient::new(shasta_base_url, shasta_root_cert.to_vec())?
    .cfs_health_check(shasta_token)
    .await
}
