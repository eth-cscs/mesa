//! End-to-end smoke test for
//! [`csm_rs::commands::delete_and_cancel_session::exec`].
//!
//! Proves the wiremock infrastructure scales from per-trait-method
//! testing (the `backend_connector.rs` suite) up to per-command-flow
//! testing. The flow here is intentionally the minimum-viable shape:
//! a CFS session with `target.definition = "image"` and no result
//! images, so the only endpoint the command needs to hit is the
//! final `DELETE /cfs/v3/sessions/{name}`. Future
//! `commands::*::exec` tests can copy this pattern and add mocks as
//! their command's flow demands.

mod common;
use common::{TEST_PEM, TEST_TOKEN};

use csm_rs::{
  cfs::v2::{CfsSessionGetResponse, Target},
  commands::delete_and_cancel_session::command as delete_and_cancel_session,
};

use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn delete_and_cancel_image_session_with_no_results_only_hits_cfs_delete()
{
  let server = MockServer::start().await;

  // The only endpoint the command should reach: the final
  // `cfs_session_v3_delete`. `.expect(1)` will fail the test if the
  // command instead tried to hit `cfs_component_v3_get_options`,
  // `ims_image_delete`, or any other endpoint we didn't anticipate.
  Mock::given(method("DELETE"))
    .and(path("/cfs/v3/sessions/sess-to-delete"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(204))
    .expect(1)
    .mount(&server)
    .await;

  // A v2 CfsSessionGetResponse the command can consume. The target
  // definition decides which branch the command takes; `"image"`
  // (with no results) skips both the `dynamic` retry-policy path
  // and the `delete_images` loop, leaving only the trailing
  // session-delete.
  let cfs_session = CfsSessionGetResponse {
    name: "sess-to-delete".to_string(),
    configuration: None,
    ansible: None,
    target: Some(Target {
      definition: Some("image".to_string()),
      groups: None,
    }),
    status: None,
    tags: None,
  };

  let client =
    csm_rs::ShastaClient::new(server.uri(), TEST_PEM.as_bytes())
      .expect("client construction");

  delete_and_cancel_session::exec(
    &client,
    TEST_TOKEN,
    Vec::new(),    // no HSM groups available — no group-based xname expansion
    &cfs_session,
    &[],           // no CFS components
    &[],           // no BSS boot parameters
    false,         // dry_run = false: we *want* the DELETE to fire
  )
  .await
  .expect("delete_and_cancel_session::exec ok");
}
