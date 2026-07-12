//! Thin wrapper bridging the generated PCS client to the public
//! `ShastaClient` API. Mirrors `crate::hsm::wrapper` — see its
//! module-level docs for the design rationale.
//!
//! Responsibilities:
//!  - construct a per-call generated `Client` with bearer auth baked in;
//!  - override the spec's basePath with the URL shape csm-rs has always
//!    used (`{base_url}/power-control/v1`);
//!  - map `progenitor_client::Error<T>` into `crate::error::Error`.
//!
//! Per-resource wrapper files (`power_cap.rs`, `power_status.rs`,
//! `transitions.rs`) hold `impl ShastaClient { pub async fn pcs_*() }`
//! blocks that delegate to the generated client via the `run` adapter.
//! No version split: PCS exposes a single API version.

use crate::{ShastaClient, error::Error, pcs::generated};

pub(crate) fn gen_client(
  client: &ShastaClient,
  token: &str,
) -> Result<generated::Client, Error> {
  let inner = crate::common::http::build_client_with_auth(
    client.root_cert(),
    Some(token),
  )?;
  let baseurl = format!("{}/power-control/v1", client.base_url());
  Ok(generated::Client::new_with_client(&baseurl, inner))
}

#[allow(clippy::enum_glob_use, clippy::match_same_arms)]
pub(crate) async fn map_err<E: std::fmt::Debug>(
  err: progenitor_client::Error<E>,
) -> Error {
  use progenitor_client::Error::*;
  match err {
    InvalidRequest(s) => Error::Message(format!("PCS invalid request: {s}")),
    CommunicationError(e) => Error::NetError(e),
    InvalidUpgrade(e) => Error::NetError(e),
    ErrorResponse(rv) => {
      let status = rv.status();
      Error::Message(format!(
        "PCS error response: status={status} body={:?}",
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
        "PCS unexpected response: status={status} url={url} body={body}"
      ))
    }
    PreHookError(s) => Error::Message(format!("PCS pre-hook error: {s}")),
  }
}

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

mod power_cap;
mod power_status;
mod transitions;
