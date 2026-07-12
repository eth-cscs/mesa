//! Minimal example: construct a [`ShastaClient`] and list every HSM
//! group on the system.
//!
//! Set the following environment variables before running:
//!
//! - `CSM_BASE_URL` — e.g. `https://api.shasta.example.com`
//! - `CSM_TOKEN`    — bearer token (Keycloak JWT)
//! - `CSM_ROOT_CERT_PATH` — path to a PEM-encoded CSM root certificate
//!
//! Run with:
//!
//! ```sh
//! cargo run --example list_hsm_groups
//! ```

use csm_rs::ShastaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let base_url = std::env::var("CSM_BASE_URL")?;
  let token = std::env::var("CSM_TOKEN")?;
  let root_cert_path = std::env::var("CSM_ROOT_CERT_PATH")?;
  let root_cert = std::fs::read(&root_cert_path)?;

  let client = ShastaClient::new(base_url, root_cert)?;

  let groups = client.hsm_group_get_all(&token).await?;

  println!("Found {} HSM group(s):", groups.len());
  for g in &groups {
    println!("- {}", g.label);
  }

  Ok(())
}
