//! Shared helpers for `ShastaClient` wiremock integration tests.

use csm_rs::ShastaClient;

/// Self-signed PEM that `reqwest::Certificate::from_pem` accepts; only used
/// to satisfy the `ShastaClient::new` contract. The mock server itself runs on
/// plain HTTP, so this cert is never actually exercised.
pub const TEST_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
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

/// Default bearer token used by all wiremock tests.
pub const TEST_TOKEN: &str = "test-token";

// Some integration-test crates don't call this (e.g. the dispatcher
// smoke tests in `tests/backend_connector.rs` construct a `Csm`
// directly), so the `dead_code` lint trips per crate. Allow it here.
#[allow(dead_code)]
pub fn make_client(base_url: &str) -> ShastaClient {
  ShastaClient::new(base_url, TEST_PEM.as_bytes().to_vec())
    .expect("ShastaClient::new should succeed with valid PEM")
}
