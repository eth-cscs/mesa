//! Thin wrapper bridging the generated HSM client to the public
//! `ShastaClient` API. Responsibilities:
//!  - construct a per-call generated `Client` with bearer auth baked in;
//!  - override the spec's basePath with the URL shape csm-rs has always used;
//!  - map `progenitor_client::Error<T>` into `crate::error::Error`.
//!
//! Per-resource wrapper files (`group.rs`, `memberships.rs`, …) hold
//! `impl ShastaClient { pub async fn hsm_*() }` blocks that delegate
//! to the generated client.

use crate::{ShastaClient, error::Error, hsm::generated};

mod component;
mod component_status;
mod ethernet_interfaces;
mod group;
mod hw_component;
pub(crate) mod hw_component_types;
mod memberships;
mod redfish_endpoint;
mod service_values;

/// Build a generated HSM `Client` bound to the caller's token.
///
/// Returns `Result<_, Error>`: bearer tokens containing bytes that are
/// not valid in an HTTP header value (control characters, `\n`, etc.)
/// surface as `Error::Message` rather than a panic.
///
/// TLS / connect-timeout / request-timeout configuration is
/// delegated to [`crate::common::http::build_client_with_auth`] so the
/// wrapper stays in lockstep with the rest of csm-rs. There is no
/// shared connection pool with `ShastaClient.http` — `reqwest::Client`
/// doesn't allow inserting default headers post-build, so we accept a
/// fresh pool per call. Threading a per-request auth hook through a
/// future progenitor version is the path to keeping a single shared
/// pool; that's out of scope for this skeleton.
pub(crate) fn gen_client(
  client: &ShastaClient,
  token: &str,
) -> Result<generated::Client, Error> {
  let inner = crate::common::http::build_client_with_auth(
    client.root_cert(),
    Some(token),
  )?;
  // Override spec basePath: csm-rs's `base_url` already ends in `/apis`.
  let baseurl = format!("{}/smd/hsm/v2", client.base_url());
  Ok(generated::Client::new_with_client(&baseurl, inner))
}

/// Map a generated `Error` into the crate's `Error` enum.
///
/// **Async**: `UnexpectedResponse` and `ErrorResponse` carry a
/// `reqwest::Response` whose body is the most useful diagnostic, and
/// reading it is an async operation. The body is read with
/// `.text().await` and included verbatim in the resulting
/// `Error::Message`. With the Task 0 spec patches stripping typed
/// error bodies, `UnexpectedResponse` becomes the common case for any
/// non-2xx, so a body-bearing diagnostic is essential here, not nice
/// to have.
///
/// The arm coverage is intentionally exhaustive (no `_` catch-all) so
/// a future progenitor bump that adds a variant forces an explicit
/// decision here instead of silently collapsing to a generic message.
/// The variant names below are taken verbatim from
/// `progenitor_client::Error<E>` in progenitor-client 0.8 (see
/// Section C of the progenitor output reference doc captured in Task 0).
///
/// `clippy::enum_glob_use` is allowed because spelling out all eight
/// variants in the `use` line would defeat the readability gain of the
/// flat match below. `clippy::match_same_arms` is allowed because the
/// arms are distinct progenitor variants that happen to map to the
/// same crate error today; collapsing them would lose the exhaustive
/// coverage that the comment above describes.
#[allow(clippy::enum_glob_use, clippy::match_same_arms)]
pub(crate) async fn map_err<E: std::fmt::Debug>(
  err: progenitor_client::Error<E>,
) -> Error {
  use progenitor_client::Error::*;
  match err {
    InvalidRequest(s) => Error::Message(format!("HSM invalid request: {s}")),
    CommunicationError(e) => Error::NetError(e),
    InvalidUpgrade(e) => Error::NetError(e),
    ErrorResponse(rv) => {
      let status = rv.status();
      // The patched HSM spec strips typed error bodies, so the inner is
      // typically `()`. Keep the formatter so the function stays generic
      // across `E`, and log the status.
      Error::Message(format!(
        "HSM error response: status={status} body={:?}",
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
        "HSM unexpected response: status={status} url={url} body={body}"
      ))
    }
    PreHookError(s) => Error::Message(format!("HSM pre-hook error: {s}")),
  }
}

/// Adapter so per-resource wrappers can write:
///
/// ```ignore
/// let rv = run(self, token, |c| async move { c.do_something().await }).await?;
/// ```
///
/// without having to spell out the match + await dance for the async
/// `map_err`. `op` receives the generated `Client` by value (it is
/// cheap to construct per call) and returns a future producing a
/// `progenitor_client::ResponseValue<T>`; on success the inner value
/// is unwrapped, on error `map_err` is awaited.
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

#[cfg(test)]
mod tests {
  use super::*;

  /// `gen_client` previously panicked on a token containing bytes that
  /// are not valid in an HTTP header value. Confirm we now surface that
  /// as a recoverable `Error::Message`.
  #[test]
  fn gen_client_with_invalid_token_returns_error() {
    // Build a minimal ShastaClient. We only need root_cert / base_url
    // to be reachable; the cert can be the test PEM used in
    // common::http tests.
    const TEST_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIBhTCCASugAwIBAgIQIRi6zePL6mKjOipn+dNuaTAKBggqhkjOPQQDAjASMRAw\n\
DgYDVQQKEwdBY21lIENvMB4XDTE3MTAyMDE5NDMwNloXDTE4MTAyMDE5NDMwNlow\n\
EjEQMA4GA1UEChMHQWNtZSBDbzBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABD0d\n\
7VNhbWvZLWPuj/RtHFjvtJBEwOkhbN/BnnE8rnZR8+sbwnc/KhCk3FhnpHZnQz7B\n\
5aETbbIgmuvewdjvSBSjYzBhMA4GA1UdDwEB/wQEAwICpDATBgNVHSUEDDAKBggr\n\
BgEFBQcDATAPBgNVHRMBAf8EBTADAQH/MCkGA1UdEQQiMCCCDmxvY2FsaG9zdDo1\n\
NDUzgg4xMjcuMC4wLjE6NTQ1MzAKBggqhkjOPQQDAgNIADBFAiEA2zpJEPQyz6/l\n\
Wf86aX6PepsntZv2GYlA5UpabfT2EZICICpJ5h/iI+i341gBmLiAFQOyTDT+/wQc\n\
6MF9+Yw1Yy0t\n\
-----END CERTIFICATE-----\n";

    let sc = ShastaClient::new(
      "https://example.invalid/apis".to_string(),
      TEST_PEM.as_bytes().to_vec(),
    )
    .expect("ShastaClient::new should accept the test PEM");

    // Newline in the token => not a valid header value.
    let result = gen_client(&sc, "bad\ntoken");
    match result {
      Err(Error::Message(m)) => {
        assert!(
          m.contains("invalid bearer token"),
          "expected message to mention 'invalid bearer token', got: {m}"
        );
      }
      other => panic!("expected Err(Error::Message), got {other:?}"),
    }
  }

  /// `map_err` on `UnexpectedResponse` must surface the response body
  /// verbatim — that's the whole point of making it async.
  #[tokio::test]
  async fn map_err_unexpected_response_includes_body() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/boom"))
      .respond_with(
        ResponseTemplate::new(500)
          .set_body_string("upstream said 'database is on fire'"),
      )
      .mount(&server)
      .await;

    let resp = reqwest::get(format!("{}/boom", server.uri()))
      .await
      .expect("request should reach the mock server");

    // Construct an UnexpectedResponse with our reqwest::Response and run
    // through map_err with an arbitrary inner error type.
    let err: progenitor_client::Error<()> =
      progenitor_client::Error::UnexpectedResponse(resp);
    let mapped = map_err(err).await;

    match mapped {
      Error::Message(m) => {
        assert!(
          m.contains("status=500"),
          "expected status=500 in message, got: {m}"
        );
        assert!(
          m.contains("database is on fire"),
          "expected response body verbatim in message, got: {m}"
        );
      }
      other => panic!("expected Error::Message, got {other:?}"),
    }
  }
}
