//! List recent CFS sessions via the v3 API.
//!
//! Set `CSM_BASE_URL`, `CSM_TOKEN`, and `CSM_ROOT_CERT_PATH`, then:
//!
//! ```sh
//! cargo run --example list_cfs_sessions
//! ```

use csm_rs::ShastaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let base_url = std::env::var("CSM_BASE_URL")?;
  let token = std::env::var("CSM_TOKEN")?;
  let root_cert = std::fs::read(std::env::var("CSM_ROOT_CERT_PATH")?)?;

  let client = ShastaClient::new(base_url, root_cert)?;

  let sessions = client
    .cfs_session_v3_get(
      &token,
      None,     // session_name
      Some(20), // limit
      None,     // after_id
      None,     // min_age
      None,     // max_age
      None,     // status
      None,     // name_contains
      None,     // is_succeeded
      None,     // tags
    )
    .await?;

  println!("Latest {} CFS session(s):", sessions.len());
  for s in &sessions {
    println!("- {}", s.name);
  }

  Ok(())
}
