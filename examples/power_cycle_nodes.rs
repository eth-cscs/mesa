//! Power-cycle a set of xnames through PCS, blocking until the
//! transition completes.
//!
//! Set `CSM_BASE_URL`, `CSM_TOKEN`, `CSM_ROOT_CERT_PATH`, and pass the
//! xnames as positional arguments:
//!
//! ```sh
//! cargo run --example power_cycle_nodes -- x1000c0s0b0n0 x1000c0s0b0n1
//! ```
//!
//! Operation defaults to `soft-restart`; override with
//! `CSM_PCS_OPERATION=off` (or any value accepted by PCS).

use csm_rs::ShastaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let base_url = std::env::var("CSM_BASE_URL")?;
  let token = std::env::var("CSM_TOKEN")?;
  let root_cert = std::fs::read(std::env::var("CSM_ROOT_CERT_PATH")?)?;
  let operation = std::env::var("CSM_PCS_OPERATION")
    .unwrap_or_else(|_| "soft-restart".to_string());

  let xnames: Vec<String> = std::env::args().skip(1).collect();
  if xnames.is_empty() {
    return Err("usage: power_cycle_nodes <xname> [<xname>...]".into());
  }

  let client = ShastaClient::new(base_url, root_cert)?;

  let response = client
    .pcs_transitions_post_block(&token, &operation, &xnames)
    .await?;

  println!("Transition completed: {response:#?}");

  Ok(())
}
