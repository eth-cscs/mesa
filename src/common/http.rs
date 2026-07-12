//! Internal HTTP helpers shared by all `*::http_client` modules.
//!
//! Centralizes the three patterns that were duplicated across ~30 files:
//!   1. Building a `reqwest::Client` with the CSM root cert and optional SOCKS5 proxy.
//!   2. Issuing a bearer-authenticated request.
//!   3. Branching on response status: deserialize success body as `T`, or map
//!      a non-success status to `Error::CsmError(Value)`.
//!
//! This module is `pub(crate)` — it intentionally does not change csm-rs's
//! public API. Existing `pub async fn x(shasta_token, shasta_base_url, ...)`
//! free functions delegate here, but their signatures stay the same.
//!
//! ## Retry policy
//!
//! `get_json` and `get_json_with_query` retry on `Error::CsmError` with a
//! 5xx status (`500..=599`) up to [`HTTP_5XX_RETRY_ATTEMPTS`] total
//! attempts, with exponential backoff starting at
//! [`HTTP_5XX_RETRY_INITIAL_DELAY`]. `post_json`, `put_json`, and `delete`
//! do **not** retry — automatic retry of non-idempotent verbs would risk
//! double-creating / double-deleting resources. Callers that need
//! at-most-once-or-error semantics for a write should compose their own
//! retry-with-idempotency-key wrapper.

use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::Error;

/// TCP connect deadline for `reqwest::Client`s built by csm-rs. A
/// long-running Manta server inheriting reqwest's default (no
/// `connect_timeout`) would stall indefinitely on a hung upstream.
pub(crate) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_mins(45);

/// Per-request deadline (response must arrive within this). Without
/// it, a CSM endpoint that accepts the connection and then hangs
/// mid-response would block the caller indefinitely. Sized to be
/// liberal enough for slow CSM dumps (`hsm_*_get_all`, full-cluster
/// inventory queries) but short enough to surface a hung peer.
pub(crate) const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_mins(15);

/// Total number of attempts (including the first) made by
/// [`retry_on_5xx`] before propagating the last 5xx error to the
/// caller. The convenience helpers `get_json` and
/// `get_json_with_query` use this.
pub(crate) const HTTP_5XX_RETRY_ATTEMPTS: u32 = 3;

/// First sleep duration between retry attempts. Doubles each attempt,
/// capped at 8 seconds, with the rationale that a transient 5xx
/// usually clears within seconds and a sustained one shouldn't keep
/// the caller waiting longer than ~14 s in total.
pub(crate) const HTTP_5XX_RETRY_INITIAL_DELAY: Duration =
  Duration::from_millis(500);

/// Retry `op` while it returns `Err(Error::CsmError { status, .. })`
/// with a 5xx `status`. Other errors (network, CSM 4xx, our own
/// structured shape errors) propagate immediately. Used internally
/// by the GET-shaped helpers — applying it to POST/PUT/DELETE would
/// risk double-creating or double-deleting, so write-shaped helpers
/// don't use it.
pub(crate) async fn retry_on_5xx<F, Fut, T>(mut op: F) -> Result<T, Error>
where
  F: FnMut() -> Fut,
  Fut: std::future::Future<Output = Result<T, Error>>,
{
  let mut delay = HTTP_5XX_RETRY_INITIAL_DELAY;
  let mut last_err: Option<Error> = None;
  for attempt in 0..HTTP_5XX_RETRY_ATTEMPTS {
    match op().await {
      Ok(v) => return Ok(v),
      Err(e) => {
        let retry = matches!(
          &e,
          Error::CsmError { status, .. } if (500..600).contains(status)
        );
        if !retry || attempt + 1 >= HTTP_5XX_RETRY_ATTEMPTS {
          return Err(e);
        }
        log::debug!(
          "retry_on_5xx: attempt {}/{} got {e}; sleeping {:?}",
          attempt + 1,
          HTTP_5XX_RETRY_ATTEMPTS,
          delay
        );
        last_err = Some(e);
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(8));
      }
    }
  }
  // Loop exit only happens if `op` was never called (impossible since
  // `HTTP_5XX_RETRY_ATTEMPTS > 0`). The `last_err` path is unreachable
  // for the same reason but the compiler can't see that.
  Err(last_err.unwrap_or_else(|| {
    Error::Message("retry_on_5xx exhausted with no attempt".to_string())
  }))
}

/// Build a `reqwest::Client` configured with the CSM root certificate.
/// This is the per-request setup that used to be inlined at every call site.
pub(crate) fn build_client(
  shasta_root_cert: &[u8],
) -> Result<reqwest::Client, Error> {
  build_client_with_auth(shasta_root_cert, None)
}

/// Build a `reqwest::Client` like [`build_client`], optionally baking in a
/// bearer-auth default header. The bearer-token variant is used by the
/// generated HSM client, where progenitor's `Client` newtype owns the
/// `reqwest::Client` and there's no convenient hook for per-request auth.
///
/// Returns `Error::Message` if `bearer_token` contains bytes that are not
/// valid in an HTTP header value (e.g. control characters, `\n`).
pub(crate) fn build_client_with_auth(
  shasta_root_cert: &[u8],
  bearer_token: Option<&str>,
) -> Result<reqwest::Client, Error> {
  let mut builder = reqwest::Client::builder()
    .connect_timeout(HTTP_CONNECT_TIMEOUT)
    .timeout(HTTP_REQUEST_TIMEOUT)
    .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

  if let Some(token) = bearer_token {
    let mut headers = reqwest::header::HeaderMap::new();
    let auth = format!("Bearer {token}");
    let mut value = reqwest::header::HeaderValue::from_str(&auth)
      .map_err(|e| Error::Message(format!("invalid bearer token: {e}")))?;
    value.set_sensitive(true);
    headers.insert(reqwest::header::AUTHORIZATION, value);
    builder = builder.default_headers(headers);
  }

  let client = builder.build()?;

  Ok(client)
}

/// On a 2xx response, deserialize the body as `T`. On any other status,
/// deserialize the body as `serde_json::Value` and return `Error::CsmError`
/// stamped with `method` and the response URL for log-correlation.
pub(crate) async fn handle_json_response<T: DeserializeOwned>(
  response: reqwest::Response,
  method: &str,
) -> Result<T, Error> {
  if response.status().is_success() {
    response.json::<T>().await.map_err(Error::NetError)
  } else {
    let status = response.status().as_u16();
    let url = response.url().to_string();
    let payload = response.json::<Value>().await.map_err(Error::NetError)?;
    Err(Error::csm_from_response(method, &url, status, payload))
  }
}

/// On a 2xx response, deserialize the body as `T`. On any other status,
/// read the body as text and return `Error::Message`. Used by endpoints
/// (mostly CFS v3 and BSS) whose error payloads are plain text, not JSON.
pub(crate) async fn handle_json_or_text_response<T: DeserializeOwned>(
  response: reqwest::Response,
) -> Result<T, Error> {
  if response.status().is_success() {
    response.json::<T>().await.map_err(Error::NetError)
  } else {
    let text = response.text().await.map_err(Error::NetError)?;
    Err(Error::Message(text))
  }
}

/// GET `url` with bearer auth, deserialize success body as `T`.
/// Retries transparently on CSM 5xx errors per the module-level
/// retry policy.
pub(crate) async fn get_json<T: DeserializeOwned>(
  client: &reqwest::Client,
  url: &str,
  shasta_token: &str,
) -> Result<T, Error> {
  retry_on_5xx(|| async {
    let response = client
      .get(url)
      .bearer_auth(shasta_token)
      .send()
      .await
      .map_err(Error::NetError)?;
    handle_json_response(response, "GET").await
  })
  .await
}

/// POST JSON `body` to `url` with bearer auth, deserialize success body as `T`.
pub(crate) async fn post_json<B, T>(
  client: &reqwest::Client,
  url: &str,
  shasta_token: &str,
  body: &B,
) -> Result<T, Error>
where
  B: Serialize + ?Sized,
  T: DeserializeOwned,
{
  let response = client
    .post(url)
    .json(body)
    .bearer_auth(shasta_token)
    .send()
    .await
    .map_err(Error::NetError)?;

  handle_json_response(response, "POST").await
}

/// PUT JSON `body` to `url` with bearer auth, deserialize success body as `T`.
pub(crate) async fn put_json<B, T>(
  client: &reqwest::Client,
  url: &str,
  shasta_token: &str,
  body: &B,
) -> Result<T, Error>
where
  B: Serialize + ?Sized,
  T: DeserializeOwned,
{
  let response = client
    .put(url)
    .json(body)
    .bearer_auth(shasta_token)
    .send()
    .await
    .map_err(Error::NetError)?;

  handle_json_response(response, "PUT").await
}

/// GET `url` with bearer auth and a query string, deserialize success body as `T`.
/// `query` is anything `serde_urlencoded` can serialize, e.g. `&[("limit", 100000)]`.
/// Retries transparently on CSM 5xx errors per the module-level
/// retry policy.
pub(crate) async fn get_json_with_query<Q, T>(
  client: &reqwest::Client,
  url: &str,
  shasta_token: &str,
  query: &Q,
) -> Result<T, Error>
where
  Q: Serialize + ?Sized,
  T: DeserializeOwned,
{
  retry_on_5xx(|| async {
    let response = client
      .get(url)
      .query(query)
      .bearer_auth(shasta_token)
      .send()
      .await
      .map_err(Error::NetError)?;
    handle_json_response(response, "GET").await
  })
  .await
}

/// On a 2xx response, deserialize the body as `T`. On `UNAUTHORIZED`, return
/// `Error::RequestError { response, payload: text }`. On any other status,
/// deserialize the body as JSON and return `Error::CsmError`. This is the
/// pattern used by HSM endpoints that distinguish auth failures from other
/// API errors.
pub(crate) async fn handle_json_or_request_error<T: DeserializeOwned>(
  response: reqwest::Response,
  method: &str,
) -> Result<T, Error> {
  if let Err(e) = response.error_for_status_ref() {
    return Err(into_request_or_json_csm_error(e, response, method).await);
  }

  response.json().await.map_err(Error::NetError)
}

/// Variant of [`handle_json_or_request_error`] that discards the success
/// body. Use for POST/PUT/DELETE endpoints whose success path is `()` and
/// whose error path follows the HSM 401-RequestError + JSON-CsmError
/// contract (HSM `/State/Components` POST/PUT, etc.).
pub(crate) async fn handle_unit_or_request_error(
  response: reqwest::Response,
  method: &str,
) -> Result<(), Error> {
  if let Err(e) = response.error_for_status_ref() {
    return Err(into_request_or_json_csm_error(e, response, method).await);
  }
  Ok(())
}

/// Variant of [`handle_json_or_request_error`] where the non-401 error path
/// reads the body as TEXT (not JSON) and returns `Error::csm_text_from_response`.
/// Used by HSM `/groups` GETs where the error body is plain text.
pub(crate) async fn handle_json_or_request_error_text<T: DeserializeOwned>(
  response: reqwest::Response,
  method: &str,
) -> Result<T, Error> {
  if let Err(e) = response.error_for_status_ref() {
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
      let url = response.url().to_string();
      let payload = response.text().await.map_err(Error::NetError)?;
      return Err(Error::RequestError {
        response: e,
        url,
        payload,
      });
    }
    let status = response.status().as_u16();
    let url = response.url().to_string();
    let payload = response.text().await.map_err(Error::NetError)?;
    return Err(Error::csm_text_from_response(method, &url, status, payload));
  }

  response.json().await.map_err(Error::NetError)
}

/// Shared error-conversion body for the 401-RequestError / non-401-JSON-CsmError
/// contract used by HSM mutating endpoints. Called by both
/// [`handle_json_or_request_error`] and [`handle_unit_or_request_error`].
async fn into_request_or_json_csm_error(
  request_err: reqwest::Error,
  response: reqwest::Response,
  method: &str,
) -> Error {
  let status = response.status();
  let url = response.url().to_string();
  if status == reqwest::StatusCode::UNAUTHORIZED {
    let payload = match response.text().await {
      Ok(p) => p,
      Err(e) => return Error::NetError(e),
    };
    return Error::RequestError {
      response: request_err,
      url,
      payload,
    };
  }
  let payload = match response.json::<Value>().await {
    Ok(p) => p,
    Err(e) => return Error::NetError(e),
  };
  Error::csm_from_response(method, &url, status.as_u16(), payload)
}

/// Run `f` across `items.chunks(chunk_size)` with at most
/// `max_in_flight` tasks active at once and flatten the per-batch
/// results into a single `Vec<U>`.
///
/// Each chunk is owned (`Vec<T>`) so the closure can be moved into a
/// `tokio::spawn`'d task without borrowing from the caller. The
/// closure is `Clone` so the helper can hand a fresh copy to each
/// spawned task.
///
/// Errors short-circuit: the first failing batch returns its error
/// (other in-flight batches are dropped when the `JoinSet` is dropped).
pub(crate) async fn parallel_batch<T, U, F, Fut>(
  items: &[T],
  chunk_size: usize,
  max_in_flight: usize,
  f: F,
) -> Result<Vec<U>, Error>
where
  T: Clone + Send + 'static,
  U: Send + 'static,
  F: Fn(Vec<T>) -> Fut + Clone + Send + 'static,
  Fut: std::future::Future<Output = Result<Vec<U>, Error>> + Send + 'static,
{
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::time::Instant;
  use tokio::sync::Semaphore;

  // Compute the batch count up front so per-batch logs can show
  // "batch X/Y" and a meaningful "pending" remainder. `div_ceil`
  // avoids an off-by-one when `items.len() % chunk_size != 0`.
  let total_batches = items.len().div_ceil(chunk_size.max(1));
  let start = Instant::now();
  log::info!(
    "parallel_batch: starting {total_batches} batches (chunk_size={chunk_size}, window={max_in_flight})"
  );

  let sem = Arc::new(Semaphore::new(max_in_flight));
  // `in_flight` is our own accounting — the Semaphore's available
  // count isn't easily queryable, and we want a value we can log on
  // both enter and exit. Increment happens inside the spawned task
  // (after the permit is owned), decrement happens just before the
  // task returns. SeqCst is overkill for what's effectively a
  // monotonic counter but matches the existing project convention
  // of "use the strongest ordering unless profiling says otherwise."
  let in_flight = Arc::new(AtomicUsize::new(0));
  let mut tasks = tokio::task::JoinSet::new();

  for (idx, chunk) in items.chunks(chunk_size).enumerate() {
    let chunk = chunk.to_vec();
    let f = f.clone();
    let permit = sem
      .clone()
      .acquire_owned()
      .await
      .expect("semaphore should not be closed");

    let batch_id = idx + 1;
    let in_flight = in_flight.clone();
    tasks.spawn(async move {
      let _permit = permit;
      let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
      let pending = total_batches.saturating_sub(batch_id);
      log::debug!(
        "parallel_batch: batch {batch_id}/{total_batches} started (in_flight={now}/{max_in_flight}, pending={pending})"
      );
      let result = f(chunk).await;
      let after = in_flight.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
      log::debug!(
        "parallel_batch: batch {batch_id}/{total_batches} done (in_flight={after}/{max_in_flight})"
      );
      result
    });
  }

  let mut out = Vec::new();
  while let Some(message) = tasks.join_next().await {
    out.append(&mut message??);
  }
  log::info!(
    "parallel_batch: completed {total_batches} batches in {:?}",
    start.elapsed()
  );
  Ok(out)
}

/// DELETE `url` with bearer auth. Returns unit on 2xx; otherwise
/// `Error::CsmError(json)`.
pub(crate) async fn delete(
  client: &reqwest::Client,
  url: &str,
  shasta_token: &str,
) -> Result<(), Error> {
  let response = client
    .delete(url)
    .bearer_auth(shasta_token)
    .send()
    .await
    .map_err(Error::NetError)?;

  if response.status().is_success() {
    Ok(())
  } else {
    let status = response.status().as_u16();
    let payload = response.json::<Value>().await.map_err(Error::NetError)?;
    Err(Error::csm_from_response("DELETE", url, status, payload))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;
  use serde_json::json;
  use wiremock::matchers::{
    bearer_token, body_json, header, method, path, query_param,
  };
  use wiremock::{Mock, MockServer, ResponseTemplate};

  // ---------- build_client ----------

  // A minimal self-signed cert, used to verify that build_client accepts
  // well-formed PEM. We don't actually make any requests against it.
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

  #[test]
  fn build_client_with_valid_pem_succeeds() {
    let client = build_client(TEST_PEM.as_bytes());
    assert!(client.is_ok());
  }

  // NOTE: there is no `build_client_with_invalid_pem_fails` test because
  // `reqwest::Certificate::from_pem` is lenient: it tolerates input without
  // PEM blocks and returns Ok with an empty cert chain. So malformed input
  // is not actually surfaced as an error by build_client.

  #[test]
  fn build_client_with_auth_invalid_token_bytes_returns_error() {
    // A `\n` byte cannot legally appear in an HTTP header value. Used to
    // panic in the old gen_client; now surfaces as Error::Message.
    let result =
      build_client_with_auth(TEST_PEM.as_bytes(), Some("bad\ntoken"));
    match result {
      Err(Error::Message(m)) => {
        assert!(
          m.contains("invalid bearer token"),
          "expected 'invalid bearer token' in message, got: {m}"
        );
      }
      other => panic!("expected Err(Error::Message), got {other:?}"),
    }
  }

  #[tokio::test]
  async fn build_client_with_auth_sends_bearer_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/ping"))
      .and(header("authorization", "Bearer token-x"))
      .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": 1})))
      .mount(&server)
      .await;

    let client =
      build_client_with_auth(TEST_PEM.as_bytes(), Some("token-x"))
        .expect("should build");
    let resp = client
      .get(format!("{}/ping", server.uri()))
      .send()
      .await
      .expect("request should reach mock");
    assert_eq!(resp.status(), 200);
  }

  // ---------- request helpers (use wiremock, plain HTTP) ----------

  #[derive(Deserialize, Debug, PartialEq)]
  struct Widget {
    id: u32,
    name: String,
  }

  #[tokio::test]
  async fn get_json_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/widgets/1"))
      .and(bearer_token("tok"))
      .respond_with(
        ResponseTemplate::new(200).set_body_json(json!({"id": 1, "name": "a"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let widget: Widget =
      get_json(&client, &format!("{}/widgets/1", server.uri()), "tok")
        .await
        .expect("should succeed");
    assert_eq!(
      widget,
      Widget {
        id: 1,
        name: "a".into()
      }
    );
  }

  #[tokio::test]
  async fn get_json_non_success_returns_csm_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/widgets/missing"))
      .respond_with(
        ResponseTemplate::new(404)
          .set_body_json(json!({"detail": "not found"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let result: Result<Widget, _> =
      get_json(&client, &format!("{}/widgets/missing", server.uri()), "tok")
        .await;
    match result {
      Err(Error::CsmError { detail, .. }) => {
        assert_eq!(detail, "not found");
      }
      other => panic!("expected CsmError, got {other:?}"),
    }
  }

  #[tokio::test]
  async fn get_json_with_query_sends_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/widgets"))
      .and(query_param("limit", "100000"))
      .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let result: Vec<Widget> = get_json_with_query(
      &client,
      &format!("{}/widgets", server.uri()),
      "tok",
      &[("limit", 100000)],
    )
    .await
    .expect("should succeed");
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn post_json_sends_body_and_deserializes_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
      .and(path("/widgets"))
      .and(bearer_token("tok"))
      .and(body_json(json!({"name": "new"})))
      .respond_with(
        ResponseTemplate::new(201)
          .set_body_json(json!({"id": 42, "name": "new"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let widget: Widget = post_json(
      &client,
      &format!("{}/widgets", server.uri()),
      "tok",
      &json!({"name": "new"}),
    )
    .await
    .expect("should succeed");
    assert_eq!(widget.id, 42);
  }

  #[tokio::test]
  async fn put_json_works() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
      .and(path("/widgets/1"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_json(json!({"id": 1, "name": "updated"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let widget: Widget = put_json(
      &client,
      &format!("{}/widgets/1", server.uri()),
      "tok",
      &json!({"name": "updated"}),
    )
    .await
    .expect("should succeed");
    assert_eq!(widget.name, "updated");
  }

  #[tokio::test]
  async fn delete_success_returns_unit() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
      .and(path("/widgets/1"))
      .and(bearer_token("tok"))
      .respond_with(ResponseTemplate::new(204))
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let result =
      delete(&client, &format!("{}/widgets/1", server.uri()), "tok").await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn delete_non_success_returns_csm_error() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
      .and(path("/widgets/locked"))
      .respond_with(
        ResponseTemplate::new(409).set_body_json(json!({"detail": "in use"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let result =
      delete(&client, &format!("{}/widgets/locked", server.uri()), "tok").await;
    match result {
      Err(Error::CsmError { detail, .. }) => {
        assert_eq!(detail, "in use");
      }
      other => panic!("expected CsmError, got {other:?}"),
    }
  }

  // ---------- response handlers ----------

  #[tokio::test]
  async fn handle_json_or_text_response_text_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/cfs"))
      .respond_with(ResponseTemplate::new(500).set_body_string("server boom"))
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let response = client
      .get(format!("{}/cfs", server.uri()))
      .send()
      .await
      .unwrap();
    let result: Result<Widget, _> =
      handle_json_or_text_response(response).await;
    match result {
      Err(Error::Message(m)) => assert_eq!(m, "server boom"),
      other => panic!("expected Message('server boom'), got {other:?}"),
    }
  }

  #[tokio::test]
  async fn handle_json_or_request_error_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/hsm"))
      .respond_with(
        ResponseTemplate::new(401).set_body_string("auth header missing"),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let response = client
      .get(format!("{}/hsm", server.uri()))
      .send()
      .await
      .unwrap();
    let result: Result<Widget, _> =
      handle_json_or_request_error(response, "GET").await;
    match result {
      Err(Error::RequestError { payload, .. }) => {
        assert_eq!(payload, "auth header missing");
      }
      other => panic!("expected RequestError, got {other:?}"),
    }
  }

  #[tokio::test]
  async fn handle_json_or_request_error_other_status_returns_csm_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/hsm/x"))
      .respond_with(
        ResponseTemplate::new(500)
          .set_body_json(json!({"detail": "db unavailable"})),
      )
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let response = client
      .get(format!("{}/hsm/x", server.uri()))
      .send()
      .await
      .unwrap();
    let result: Result<Widget, _> =
      handle_json_or_request_error(response, "GET").await;
    match result {
      Err(Error::CsmError { detail, .. }) => {
        assert_eq!(detail, "db unavailable");
      }
      other => panic!("expected CsmError, got {other:?}"),
    }
  }

  // ---------- parallel_batch ----------

  #[tokio::test]
  async fn parallel_batch_flattens_results() {
    let items: Vec<i32> = (0..10).collect();
    let out = parallel_batch(&items, 3, 4, |chunk: Vec<i32>| async move {
      Ok::<_, Error>(chunk.into_iter().map(|x| x * 2).collect::<Vec<_>>())
    })
    .await
    .expect("should succeed");
    let mut sorted = out;
    sorted.sort_unstable();
    assert_eq!(sorted, (0..10).map(|x| x * 2).collect::<Vec<_>>());
  }

  #[tokio::test]
  async fn parallel_batch_propagates_error() {
    let items: Vec<i32> = (0..5).collect();
    let result: Result<Vec<i32>, _> =
      parallel_batch(&items, 2, 2, |_chunk: Vec<i32>| async move {
        Err(Error::Message("boom".into()))
      })
      .await;
    match result {
      Err(Error::Message(m)) => assert_eq!(m, "boom"),
      other => panic!("expected Message('boom'), got {other:?}"),
    }
  }

  #[tokio::test]
  async fn parallel_batch_empty_input_returns_empty() {
    let items: Vec<i32> = vec![];
    let out: Vec<i32> =
      parallel_batch(&items, 3, 4, |_chunk: Vec<i32>| async move {
        unreachable!("closure should not be called on empty input")
      })
      .await
      .expect("should succeed");
    assert!(out.is_empty());
  }

  // Bearer auth verification — make sure every helper actually sends the token
  #[tokio::test]
  async fn bearer_token_is_sent_with_get_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/auth"))
      .and(header("authorization", "Bearer test-token"))
      .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": 1})))
      .mount(&server)
      .await;

    let client = reqwest::Client::new();
    let _: serde_json::Value =
      get_json(&client, &format!("{}/auth", server.uri()), "test-token")
        .await
        .expect("should succeed");
  }

  // ---------- retry_on_5xx ----------

  #[tokio::test]
  async fn retry_on_5xx_returns_eventual_success() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let calls = AtomicU32::new(0);
    let result: u32 = retry_on_5xx(|| async {
      let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
      if n < 3 {
        Err(Error::CsmError {
          method: "GET".into(),
          url: "http://example/x".into(),
          status: 503,
          detail: "transient".into(),
          body: None,
        })
      } else {
        Ok(42)
      }
    })
    .await
    .expect("third attempt succeeds");
    assert_eq!(result, 42);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn retry_on_5xx_propagates_after_exhausting_attempts() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let calls = AtomicU32::new(0);
    let result: Result<u32, _> = retry_on_5xx(|| async {
      calls.fetch_add(1, Ordering::SeqCst);
      Err(Error::CsmError {
        method: "GET".into(),
        url: "http://example/x".into(),
        status: 502,
        detail: "still down".into(),
        body: None,
      })
    })
    .await;
    match result {
      Err(Error::CsmError { status, .. }) => assert_eq!(status, 502),
      other => panic!("expected CsmError(502), got {other:?}"),
    }
    assert_eq!(calls.load(Ordering::SeqCst), HTTP_5XX_RETRY_ATTEMPTS);
  }

  #[tokio::test]
  async fn retry_on_5xx_does_not_retry_4xx() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let calls = AtomicU32::new(0);
    let result: Result<u32, _> = retry_on_5xx(|| async {
      calls.fetch_add(1, Ordering::SeqCst);
      Err(Error::CsmError {
        method: "GET".into(),
        url: "http://example/x".into(),
        status: 404,
        detail: "not found".into(),
        body: None,
      })
    })
    .await;
    assert!(matches!(result, Err(Error::CsmError { status: 404, .. })));
    // First attempt only — 4xx is terminal.
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn retry_on_5xx_does_not_retry_net_error() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let calls = AtomicU32::new(0);
    let result: Result<u32, _> = retry_on_5xx(|| async {
      calls.fetch_add(1, Ordering::SeqCst);
      Err(Error::Message("network down".to_string()))
    })
    .await;
    assert!(matches!(result, Err(Error::Message(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }
}
