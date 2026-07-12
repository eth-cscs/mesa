//! [`ShastaClient`] — connection-pool-owning entry point for talking to a
//! Shasta CSM API.
//!
//! Holds the base URL, root certificate, and a
//! pre-built `reqwest::Client` (with its connection pool, TLS context,
//! and DNS resolver). The bearer token is **not** stored on the client
//! — it is passed per request.
//!
//! Construct one `ShastaClient` per Shasta installation and reuse it
//! across calls; clones are cheap (`reqwest::Client` is reference-
//! counted internally).

use crate::common::http;
use crate::error::Error;

/// Connection details + a reusable `reqwest::Client` for one Shasta CSM
/// installation. Token is passed per request, not stored.
///
/// # Method naming
///
/// Methods follow the convention `<module>_<resource>_<verb>`, optionally
/// suffixed with an API version. Each method takes a bearer token as its
/// first argument. Examples:
///
/// - `ims_image_get_all(token)` — GET /ims/v3/images
/// - `cfs_configuration_v2_put(token, …)` — PUT /cfs/v2/configurations/{name}
/// - `hsm_group_delete_member(token, label, id)` — DELETE /smd/hsm/v2/groups/{label}/members/{id}
/// - `pcs_transitions_post_block(token, …)` — POST /power-control/v1/transitions and poll until completion
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), csm_rs::Error> {
/// let client = csm_rs::ShastaClient::new(
///     "https://api.shasta.example.com",
///     std::fs::read("/etc/shasta/ca.crt").unwrap(),
/// )?;
///
/// // Token is supplied per call; one client serves any number of tokens:
/// let token = "your-bearer-token";
/// let images = client.ims_image_get_all(token).await?;
/// let group = client.hsm_group_get_one(token, "zinal").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ShastaClient {
  pub(crate) base_url: String,
  pub(crate) root_cert: Vec<u8>,
  pub(crate) http: reqwest::Client,
}

impl ShastaClient {
  /// Build a new client. Constructs the underlying `reqwest::Client` once,
  /// applying the CSM root cert.
  ///
  /// # Errors
  ///
  /// Returns [`Error::NetError`] if `reqwest::Client::build` fails.
  /// Malformed PEM input is silently tolerated by
  /// `reqwest::Certificate::from_pem` (it just produces an empty trust
  /// chain), so this constructor does not surface PEM errors.
  #[must_use = "constructing a ShastaClient without using it is a no-op"]
  pub fn new(
    base_url: impl Into<String>,
    root_cert: impl Into<Vec<u8>>,
  ) -> Result<Self, Error> {
    let root_cert = root_cert.into();
    let http = http::build_client(&root_cert)?;
    Ok(Self {
      base_url: base_url.into(),
      root_cert,
      http,
    })
  }

  /// The Shasta API base URL (e.g. `https://api.shasta.example.com`).
  #[must_use]
  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  /// The PEM-encoded root certificate trusted for HTTPS calls.
  #[must_use]
  pub fn root_cert(&self) -> &[u8] {
    &self.root_cert
  }

  pub(crate) fn http(&self) -> &reqwest::Client {
    &self.http
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Reuse the test PEM from common::http tests to avoid duplicating a cert
  // blob; both modules need a syntactically valid PEM to construct a client.
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
  fn new_with_valid_pem_and_no_proxy_succeeds() {
    let client = ShastaClient::new(
      "https://api.shasta.example.com",
      TEST_PEM.as_bytes().to_vec(),
    )
    .expect("client construction should succeed");

    assert_eq!(client.base_url(), "https://api.shasta.example.com");
    assert_eq!(client.root_cert(), TEST_PEM.as_bytes());
  }

  // NOTE: there is no test for "invalid PEM fails" because
  // `reqwest::Certificate::from_pem` is lenient — see the analogous comment
  // in `common::http::tests`. Garbage input returns Ok with an empty chain.

  #[test]
  fn clone_preserves_all_fields() {
    let client = ShastaClient::new(
      "https://api.example.com",
      TEST_PEM.as_bytes().to_vec(),
    )
    .unwrap();
    let cloned = client.clone();

    assert_eq!(client.base_url(), cloned.base_url());
    assert_eq!(client.root_cert(), cloned.root_cert());
  }

  #[test]
  fn accepts_owned_and_borrowed_strings_via_into() {
    // String
    let _ = ShastaClient::new(
      "https://api.example.com".to_string(),
      TEST_PEM.as_bytes().to_vec(),
    )
    .unwrap();
    // &str
    let _ =
      ShastaClient::new("https://api.example.com", TEST_PEM.as_bytes())
        .unwrap();
  }
}
