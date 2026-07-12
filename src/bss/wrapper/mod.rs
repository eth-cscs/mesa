//! Thin wrapper bridging the generated BSS client to the public
//! `ShastaClient` API. Mirrors `crate::hsm::wrapper` and
//! `crate::cfs::wrapper` — see their module-level docs for the design
//! rationale.
//!
//! Responsibilities:
//!  - construct a per-call generated `Client` with bearer auth baked in;
//!  - override the spec's basePath with the URL shape csm-rs has always
//!    used (`{base_url}/bss`);
//!  - map `progenitor_client::Error<T>` into `crate::error::Error` (async,
//!    reads the body for UnexpectedResponse/ErrorResponse — same idiom
//!    as `crate::hsm::wrapper::map_err`).
//!
//! BSS has only one resource (`/boot/v1/bootparameters`), so all 6
//! `bss_bootparameters_*` methods live in this file directly rather
//! than in per-resource submodules.
//!
//! # Type strategy: Option B (keep hand-written `BootParameters`)
//!
//! The hand-written [`crate::bss::types::BootParameters`] is retained
//! and the generated `types::BootParams` is not exposed. Reasons:
//!
//! - [`BootParameters`](crate::bss::types::BootParameters) carries ~10
//!   instance methods (`get_boot_image`, `update_boot_image`,
//!   `apply_kernel_params`, `update_kernel_params`,
//!   `add_kernel_params`, `delete_kernel_params`, …) that operate on
//!   `self.params: String`, `self.kernel: String`, `self.initrd:
//!   String` as non-`Option`. Swapping to the generated `BootParams`
//!   (where these are `Option<String>`) would force every method body
//!   to thread `.as_deref().unwrap_or("")` and break the public type
//!   signatures used by `crate::ims::image::utils`, `crate::cfs::cleanup`,
//!   `crate::cfs::cleanup_session`, `crate::node::utils`, and
//!   `crate::backend_connector::cfs`.
//! - Generated `nids: Vec<i64>` vs hand-written `Option<Vec<u32>>` is
//!   a breaking integer-width change to the public API.
//! - The dispatcher mirror
//!   `manta_backend_dispatcher::types::bss::BootParameters` has the
//!   same shape as csm-rs's hand-written type; switching would force a
//!   coordinated dispatcher release.
//! - The spec models `cloud_init` as a typed `CloudInit` struct;
//!   csm-rs and the dispatcher both use `Option<serde_json::Value>` to
//!   stay tolerant of vendor extensions.
//!
//! # Per-method routing (all 6 stay on raw `reqwest`)
//!
//! - `bss_bootparameters_get` — STAY RAW. The generated
//!   `get_boot_parameters` takes a single `Option<&str>` `name` query
//!   param plus a mandatory `&types::BootParams` JSON body. csm-rs
//!   sends repeated `?name=X&name=Y&…` query params and no body. The
//!   wiremock test in `tests/shasta_client_misc.rs` asserts the
//!   repeated-`name=` form; routing through progenitor would change
//!   the wire shape.
//! - `bss_bootparameters_get_all` — convenience wrapper that calls
//!   `bss_bootparameters_get(&[])` internally; inherits its routing.
//! - `bss_bootparameters_get_multiple` — parallel-batched wrapper over
//!   `bss_bootparameters_get`; inherits its routing.
//! - `bss_bootparameters_put` — STAY RAW. csm-rs's public signature
//!   returns `Result<BootParameters, Error>` (the 200 body
//!   deserialised), but generated `put_boot_parameters` returns
//!   `ResponseValue<()>` per spec. Routing through progenitor would
//!   change the public return type.
//! - `bss_bootparameters_post` — STAY RAW. Option B keeps
//!   `BootParameters` as the public input type; converting to the
//!   generated `BootParams` at the boundary is friction with no
//!   wire-shape benefit since the resulting JSON is the same.
//! - `bss_bootparameters_patch` — same rationale as POST.
//!
//! The `gen_client` / `map_err` / `run` helpers are retained so a
//! future spec revision can be migrated incrementally without a
//! second scaffolding pass.

use core::result::Result;
use std::time::Instant;

use crate::{ShastaClient, bss::generated, common::http, error::Error};

use super::types::BootParameters;

#[allow(dead_code)]
pub(crate) fn gen_client(
  client: &ShastaClient,
  token: &str,
) -> Result<generated::Client, Error> {
  let inner = crate::common::http::build_client_with_auth(
    client.root_cert(),
    Some(token),
  )?;
  let baseurl = format!("{}/bss", client.base_url());
  Ok(generated::Client::new_with_client(&baseurl, inner))
}

#[allow(dead_code, clippy::enum_glob_use, clippy::match_same_arms)]
pub(crate) async fn map_err<E: std::fmt::Debug>(
  err: progenitor_client::Error<E>,
) -> Error {
  use progenitor_client::Error::*;
  match err {
    InvalidRequest(s) => Error::Message(format!("BSS invalid request: {s}")),
    CommunicationError(e) => Error::NetError(e),
    InvalidUpgrade(e) => Error::NetError(e),
    ErrorResponse(rv) => {
      let status = rv.status();
      Error::Message(format!(
        "BSS error response: status={status} body={:?}",
        rv.into_inner()
      ))
    }
    ResponseBodyError(e) => Error::NetError(e),
    InvalidResponsePayload(_, e) => Error::SerdeJsonError(e),
    UnexpectedResponse(resp) => {
      let status = resp.status();
      let url = resp.url().clone();
      let body = resp
        .text()
        .await
        .unwrap_or_else(|e| format!("<body read failed: {e}>"));
      Error::Message(format!(
        "BSS unexpected response: status={status} url={url} body={body}"
      ))
    }
    PreHookError(s) => Error::Message(format!("BSS pre-hook error: {s}")),
  }
}

#[allow(dead_code)]
pub(crate) async fn run<F, Fut, T, E>(
  client: &ShastaClient,
  token: &str,
  op: F,
) -> Result<T, Error>
where
  F: FnOnce(generated::Client) -> Fut,
  Fut: std::future::Future<
      Output = Result<
        progenitor_client::ResponseValue<T>,
        progenitor_client::Error<E>,
      >,
    >,
  E: std::fmt::Debug,
{
  let gc = gen_client(client, token)?;
  match op(gc).await {
    Ok(rv) => Ok(rv.into_inner()),
    Err(e) => Err(map_err(e).await),
  }
}

impl ShastaClient {
  /// Get node boot params. Ref: <https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/get/>.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_get(
    &self,
    token: &str,
    xnames: &[String],
  ) -> Result<Vec<BootParameters>, Error> {
    log::debug!("Get BSS bootparameters");

    let url_api = format!("{}/bss/boot/v1/bootparameters", self.base_url());

    let params: Vec<_> = xnames.iter().map(|xname| ("name", xname)).collect();

    let response = self
      .http()
      .get(url_api)
      .query(&params)
      .bearer_auth(token)
      .send()
      .await
      .map_err(Error::NetError)?;

    http::handle_json_or_text_response(response).await
  }

  /// `GET /bss/boot/v1/bootparameters` — fetch boot parameters for
  /// every node BSS knows about.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_get_all(
    &self,
    token: &str,
  ) -> Result<Vec<BootParameters>, Error> {
    self.bss_bootparameters_get(token, &[]).await
  }

  /// `GET /bss/boot/v1/bootparameters` for many xnames, parallelised
  /// in chunks of 30 with up to 10 concurrent batches.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_get_multiple(
    &self,
    token: &str,
    xnames: &[String],
  ) -> Result<Vec<BootParameters>, Error> {
    let start = Instant::now();

    let client = self.clone();
    let token = token.to_string();
    let boot_params_vec = http::parallel_batch(xnames, 30, 10, move |chunk| {
      let client = client.clone();
      let token = token.clone();
      async move { client.bss_bootparameters_get(&token, &chunk).await }
    })
    .await?;

    log::debug!(
      "Time elapsed to get BSS bootparameters is: {:?}",
      start.elapsed()
    );
    Ok(boot_params_vec)
  }

  /// Change nodes boot params. Ref: <https://apidocs.svc.cscs.ch/iaas/bss/tag/bootparameters/paths/~1bootparameters/put/>.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_put(
    &self,
    token: &str,
    boot_parameters: BootParameters,
  ) -> Result<BootParameters, Error> {
    let api_url = format!("{}/bss/boot/v1/bootparameters", self.base_url());

    log::debug!(
      "request payload:\n{}",
      serde_json::to_string_pretty(&boot_parameters)?
    );

    let response = self
      .http()
      .put(api_url)
      .json(&boot_parameters)
      .bearer_auth(token)
      .send()
      .await
      .map_err(Error::NetError)?;

    if response.status().is_success() {
      Ok(response.json().await?)
    } else {
      Err(Error::Message(response.text().await?))
    }
  }

  /// POST a single set of `BootParameters`. Used to create new entries.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_post(
    &self,
    token: &str,
    boot_parameters: BootParameters,
  ) -> Result<(), Error> {
    let api_url = format!("{}/bss/boot/v1/bootparameters", self.base_url());

    let response = self
      .http()
      .post(api_url)
      .bearer_auth(token)
      .json(&boot_parameters)
      .send()
      .await
      .map_err(Error::NetError)?;

    if response.status().is_success() {
      Ok(())
    } else {
      Err(Error::Message(response.text().await?))
    }
  }

  /// `PATCH /bss/boot/v1/bootparameters` — partial update of an
  /// existing entry.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn bss_bootparameters_patch(
    &self,
    token: &str,
    boot_parameters: &BootParameters,
  ) -> Result<(), Error> {
    let api_url = format!("{}/bss/boot/v1/bootparameters", self.base_url());

    let response = self
      .http()
      .patch(api_url)
      .json(&boot_parameters)
      .bearer_auth(token)
      .send()
      .await
      .map_err(Error::NetError)?;

    if response.status().is_success() {
      Ok(())
    } else {
      Err(Error::Message(response.text().await?))
    }
  }
}
