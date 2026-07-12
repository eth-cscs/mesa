//! Thin wrapper bridging the generated CFS client to the public
//! `ShastaClient` API. Mirrors `crate::hsm::wrapper` — see its
//! module-level docs for the design rationale.
//!
//! Responsibilities:
//!  - construct a per-call generated `Client` with bearer auth baked in;
//!  - override the spec's basePath with the URL shape csm-rs has always
//!    used (`{base_url}/cfs` — v2 and v3 prefixes come from the
//!    operation paths);
//!  - map `progenitor_client::Error<T>` into `crate::error::Error` (async,
//!    reads the body for UnexpectedResponse/ErrorResponse — same idiom
//!    as `crate::hsm::wrapper::map_err`).
//!
//! Per-resource wrapper files (`v2/component.rs`, `v3/component.rs`,
//! etc.) hold `impl ShastaClient { pub async fn cfs_*() }` blocks that
//! delegate to the generated client via the `run` adapter.

use crate::{ShastaClient, cfs::generated, error::Error};

/// Build a generated CFS `Client` bound to the caller's token. Re-uses
/// the shared `http::build_client_with_auth` helper so timeout / TLS /
/// proxy config stays consistent with the rest of csm-rs.
pub(crate) fn gen_client(
  client: &ShastaClient,
  token: &str,
) -> Result<generated::Client, Error> {
  let inner = crate::common::http::build_client_with_auth(
    client.root_cert(),
    Some(token),
  )?;
  // CFS basePath: csm-rs's `base_url` already ends in `/apis`; CFS
  // operations live under `/cfs/...` (v2 and v3 prefixes are part of
  // the operation paths).
  let baseurl = format!("{}/cfs", client.base_url());
  Ok(generated::Client::new_with_client(&baseurl, inner))
}

/// Map a generated `Error` into the crate's `Error` enum. Async because
/// `UnexpectedResponse` and `ErrorResponse` carry a `reqwest::Response`
/// whose body must be read to produce a useful diagnostic. Mirrors
/// `crate::hsm::wrapper::map_err`.
///
/// The arm coverage is intentionally exhaustive (no `_` catch-all) so
/// a future progenitor bump that adds a variant forces an explicit
/// decision here instead of silently collapsing to a generic message.
#[allow(clippy::enum_glob_use, clippy::match_same_arms)]
pub(crate) async fn map_err<E: std::fmt::Debug>(
  err: progenitor_client::Error<E>,
) -> Error {
  use progenitor_client::Error::*;
  match err {
    InvalidRequest(s) => Error::Message(format!("CFS invalid request: {s}")),
    CommunicationError(e) => Error::NetError(e),
    InvalidUpgrade(e) => Error::NetError(e),
    ErrorResponse(rv) => {
      let status = rv.status();
      Error::Message(format!(
        "CFS error response: status={status} body={:?}",
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
        "CFS unexpected response: status={status} url={url} body={body}"
      ))
    }
    PreHookError(s) => Error::Message(format!("CFS pre-hook error: {s}")),
  }
}

/// Adapter so per-resource wrappers can write:
/// `let rv = run(self, token, |c| c.do_something()).await?;`
/// Mirrors `crate::hsm::wrapper::run`.
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

mod v2;
mod v3;
