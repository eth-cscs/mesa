//! Wiremock smoke tests for the [`csm_rs::backend_connector::Csm`]
//! dispatcher trait impls. Each test stands up a mock CSM, constructs
//! a `Csm`, calls one trait method through the dispatcher
//! interface, and asserts the request hit the expected endpoint with
//! the right bearer auth and that the wire response decodes through
//! the csm-rs -> manta-backend-dispatcher type conversion.
//!
//! This file is gated on the `manta-dispatcher` Cargo feature because
//! the dispatcher boundary itself is feature-gated.

#![cfg(feature = "manta-dispatcher")]

mod common;
use common::{TEST_PEM, TEST_TOKEN};

use csm_rs::backend_connector::Csm;
use manta_backend_dispatcher::interfaces::{
  authentication::AuthenticationTrait,
  bss::BootParametersTrait,
  cfs::CfsTrait,
  hsm::{
    component::ComponentTrait,
    component_ethernet_interface::ComponentEthernetInterfaceTrait,
    group::GroupTrait, hardware_inventory::HardwareInventory,
    redfish_endpoint::RedfishEndpointTrait,
  },
  ims::ImsTrait,
  pcs::PCSTrait,
};

use serde_json::json;
use wiremock::matchers::{
  bearer_token, body_partial_json, method, path, query_param,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_csm(base_url: &str) -> Csm {
  Csm::new(base_url, TEST_PEM.as_bytes()).expect("Csm::new ok")
}

// ---------- BootParametersTrait ----------

#[tokio::test]
async fn bss_get_all_bootparameters_hits_bss_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/bss/boot/v1/bootparameters"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
      "hosts": ["x1000c0s0b0n0"],
      "params": "console=ttyS0",
      "kernel": "s3://boot-images/abc/kernel",
      "initrd": "s3://boot-images/abc/initrd",
    }])))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let bp = csm.get_all_bootparameters(TEST_TOKEN).await.expect("ok");
  assert_eq!(bp.len(), 1);
  assert_eq!(bp[0].hosts, vec!["x1000c0s0b0n0".to_string()]);
  assert_eq!(bp[0].params, "console=ttyS0");
}

#[tokio::test]
async fn bss_get_bootparameters_forwards_node_filter() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/bss/boot/v1/bootparameters"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let bp = csm
    .get_bootparameters(TEST_TOKEN, &["x1000c0s0b0n0".to_string()])
    .await
    .expect("ok");
  assert!(bp.is_empty());
}

#[tokio::test]
async fn bss_add_bootparameters_posts_body() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/bss/boot/v1/bootparameters"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"hosts": ["x1000c0s0b0n0"]})))
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let bp = manta_backend_dispatcher::types::bss::BootParameters {
    hosts: vec!["x1000c0s0b0n0".to_string()],
    params: "console=ttyS0".to_string(),
    kernel: "s3://boot-images/abc/kernel".to_string(),
    initrd: "s3://boot-images/abc/initrd".to_string(),
    ..Default::default()
  };
  csm.add_bootparameters(TEST_TOKEN, &bp).await.expect("ok");
}

#[tokio::test]
async fn bss_delete_bootparameters_returns_not_implemented_error() {
  // No mock — the impl short-circuits to an Err without making a
  // network call. The .expect(1) discipline would catch any
  // regression that started making a request here.
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let bp = manta_backend_dispatcher::types::bss::BootParameters::default();
  let err = csm
    .delete_bootparameters(TEST_TOKEN, &bp)
    .await
    .expect_err("delete is not implemented");
  let msg = err.to_string();
  assert!(
    msg.contains("not implemented"),
    "expected 'not implemented' in {msg:?}"
  );
}

// ---------- PCSTrait ----------

#[tokio::test]
async fn pcs_transitions_get_hits_v1_transitions_by_id() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/power-control/v1/transitions/tr-123"))
    .and(bearer_token(TEST_TOKEN))
    // NOTE: PCS uses camelCase keys for the top-level fields
    // (`transitionID`, `taskCounts`, `createTime`,
    // `automaticExpirationTime`) but **kebab-case** for the
    // task-counts entries (`in-progress`, `un-supported`). Both forms
    // must be present or `serde` rejects the response.
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "transitionID": "tr-123",
      "operation": "On",
      "transitionStatus": "completed",
      "taskCounts": {
        "total": 1, "new": 0, "in-progress": 0,
        "failed": 0, "succeeded": 1, "un-supported": 0
      },
      "tasks": [],
      "createTime": "2026-01-01T00:00:00Z",
      "automaticExpirationTime": "2026-01-02T00:00:00Z",
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  // Call the trait method via FQN. `Csm` has no inherent
  // `pcs_transitions_get`; the FQN form is kept for explicitness and
  // symmetry with the other PCS test.
  let tr = PCSTrait::pcs_transitions_get(&csm, TEST_TOKEN, "tr-123")
    .await
    .expect("ok");
  assert_eq!(tr.transition_status, "completed");
}

#[tokio::test]
async fn pcs_transitions_post_hits_v1_transitions() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/power-control/v1/transitions"))
    .and(bearer_token(TEST_TOKEN))
    // The client takes `operation: &str`, parses into the `Operation`
    // enum, then re-serializes — so the request body uses the enum's
    // titlecase form ("On"), not the caller's input casing.
    .and(body_partial_json(json!({"operation": "On"})))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "transitionID": "tr-new",
      "operation": "On",
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  // NOTE: `pcs_transitions_post` takes a `&str` and parses it
  // through `Operation::from_str` (which accepts lowercase: "on",
  // "off", "soft-restart", ...). The parsed enum is then re-
  // serialised before sending, producing the wire form ("On",
  // "Off", "Soft-Restart") — that's why the `body_partial_json`
  // matcher above expects `"On"` even though we pass `"on"` here.
  let started = csm
    .pcs_transitions_post(TEST_TOKEN, "on", &["x1000c0s0b0n0".to_string()])
    .await
    .expect("ok");
  assert_eq!(started.transition_id, "tr-new");
}

// ---------- GroupTrait ----------

#[tokio::test]
async fn group_get_all_groups_hits_smd_v2_groups() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/groups"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      {"label": "zinal", "members": {"ids": ["x1000c0s0b0n0"]}},
      {"label": "ela", "members": {"ids": []}},
    ])))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let groups = csm.get_groups(TEST_TOKEN, None).await.expect("ok");
  assert_eq!(groups.len(), 2);
  assert_eq!(groups[0].label, "zinal");
}

#[tokio::test]
async fn group_get_group_hits_filter_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/groups"))
    .and(query_param("group", "zinal"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      {"label": "zinal", "members": {"ids": ["x1000c0s0b0n0"]}},
    ])))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let g = csm.get_group(TEST_TOKEN, "zinal").await.expect("ok");
  assert_eq!(g.label, "zinal");
}

#[tokio::test]
async fn group_delete_group_hits_singular_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("DELETE"))
    .and(path("/smd/hsm/v2/groups/zinal"))
    .and(bearer_token(TEST_TOKEN))
    // `HsmActionResponse.code` deserialises as i64 (matches real CSM
    // wire format — the swagger says String but the server sends an
    // integer).
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "code": 0, "message": "deleted"
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  csm.delete_group(TEST_TOKEN, "zinal").await.expect("ok");
}

#[tokio::test]
async fn group_post_member_hits_members_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/smd/hsm/v2/groups/zinal/members"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"id": "x1000c0s0b0n0"})))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "code": 0, "message": "added"
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  csm
    .post_member(TEST_TOKEN, "zinal", "x1000c0s0b0n0")
    .await
    .expect("ok");
}

// ---------- ComponentTrait ----------

#[tokio::test]
async fn component_get_all_nodes_hits_state_components() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/State/Components"))
    .and(query_param("type", "Node"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "Components": [
        {"ID": "x1000c0s0b0n0", "Type": "Node"},
      ]
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let nodes = csm.get_all_nodes(TEST_TOKEN, None).await.expect("ok");
  assert_eq!(nodes.components.as_ref().map(Vec::len), Some(1));
}

// ---------- ImsTrait ----------

#[tokio::test]
async fn ims_get_images_hits_ims_v3_images() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/ims/v3/images"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      {"id": "img-1", "name": "ubuntu-noble", "created": "2026-01-01T00:00:00Z"}
    ])))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let images = csm.get_images(TEST_TOKEN, None).await.expect("ok");
  assert_eq!(images.len(), 1);
  assert_eq!(images[0].name, "ubuntu-noble");
}

// ---------- BootParametersTrait::update (PATCH) ----------

#[tokio::test]
async fn bss_update_bootparameters_patches_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("PATCH"))
    .and(path("/bss/boot/v1/bootparameters"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"hosts": ["x1000c0s0b0n0"]})))
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let bp = manta_backend_dispatcher::types::bss::BootParameters {
    hosts: vec!["x1000c0s0b0n0".to_string()],
    params: "console=ttyS0".to_string(),
    kernel: "s3://boot-images/abc/kernel".to_string(),
    initrd: "s3://boot-images/abc/initrd".to_string(),
    ..Default::default()
  };
  csm
    .update_bootparameters(TEST_TOKEN, &bp)
    .await
    .expect("ok");
}

// ---------- PCSTrait::power_status ----------

#[tokio::test]
async fn pcs_power_status_posts_filters() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/power-control/v1/power-status"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({
      "xname": ["x1000c0s0b0n0"],
      "powerStateFilter": "on",
      "managementStateFilter": "available",
    })))
    // NOTE: PCS `PowerStatus` requires *both* `supportedPowerTransitions`
    // (an array, possibly empty) and `lastUpdated` (camelCase) on every
    // entry. Omitting either rejects deserialisation; an early mock
    // shape that dropped them silently produced "missing field" errors
    // that were easy to mistake for a routing bug.
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "status": [{
        "xname": "x1000c0s0b0n0",
        "powerState": "on",
        "managementState": "available",
        "supportedPowerTransitions": [],
        "lastUpdated": "2026-01-01T00:00:00Z",
      }]
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let status = csm
    .power_status(
      TEST_TOKEN,
      &["x1000c0s0b0n0".to_string()],
      Some("on"),
      Some("available"),
    )
    .await
    .expect("ok");
  assert_eq!(status.status.len(), 1);
}

// ---------- GroupTrait — more shapes ----------

#[tokio::test]
async fn group_add_group_posts_to_groups_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/smd/hsm/v2/groups"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"label": "zinal"})))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "code": 0, "message": "added"
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let group = manta_backend_dispatcher::types::Group {
    label: "zinal".to_string(),
    description: None,
    tags: None,
    exclusive_group: None,
    members: None,
  };
  let returned = csm.add_group(TEST_TOKEN, group).await.expect("ok");
  assert_eq!(returned.label, "zinal");
}

#[tokio::test]
async fn group_delete_member_from_group_hits_member_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("DELETE"))
    .and(path("/smd/hsm/v2/groups/zinal/members/x1000c0s0b0n0"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  csm
    .delete_member_from_group(TEST_TOKEN, "zinal", "x1000c0s0b0n0")
    .await
    .expect("ok");
}

// ---------- ComponentTrait::get with one filter ----------

#[tokio::test]
async fn component_get_forwards_role_query_param() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/State/Components"))
    .and(query_param("role", "Compute"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "Components": []
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let arr = csm
    .get(
      TEST_TOKEN,
      None,
      None,
      None,
      None,
      Some("Compute"),
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
      None,
    )
    .await
    .expect("ok");
  assert!(arr.components.unwrap_or_default().is_empty());
}

// ---------- HardwareInventory ----------

#[tokio::test]
async fn hw_inventory_get_query_hits_query_path() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/Inventory/Hardware/Query/x1000c0s0b0n0"))
    .and(bearer_token(TEST_TOKEN))
    // Empty inventory is a valid response shape; the test exercises
    // URL routing and bearer forwarding, not field coverage.
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let _ = csm
    .get_inventory_hardware_query(
      TEST_TOKEN,
      "x1000c0s0b0n0",
      None,
      None,
      None,
      None,
      None,
    )
    .await
    .expect("ok");
}

// ---------- RedfishEndpointTrait stubs (no network) ----------

#[tokio::test]
async fn redfish_get_all_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let err = csm
    .get_all_redfish_endpoints(TEST_TOKEN)
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}

#[tokio::test]
async fn redfish_delete_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let err = csm
    .delete_redfish_endpoint(TEST_TOKEN, "x1000c0s0b0n0")
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}

// ---------- CfsTrait ----------

#[tokio::test]
async fn cfs_get_configuration_hits_v3_configurations() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/cfs/v3/configurations"))
    .and(bearer_token(TEST_TOKEN))
    // NOTE: CFS v3 configuration responses use `last_updated`
    // (snake_case) — in contrast to PCS v1, which uses `lastUpdated`
    // (camelCase). The two services were written by different teams
    // and never reconciled.
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "configurations": [
        {"name": "cfg-1", "last_updated": "2026-01-01T00:00:00Z", "layers": []}
      ],
      "next": null
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let configs = csm.get_configuration(TEST_TOKEN, None).await.expect("ok");
  assert_eq!(configs.len(), 1);
  assert_eq!(configs[0].name, "cfg-1");
}

#[tokio::test]
async fn cfs_get_sessions_hits_v3_sessions() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/cfs/v3/sessions"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "sessions": [
        {"name": "sess-1", "debug_on_failure": false}
      ],
      "next": null
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let sessions = csm
    .get_sessions(
      TEST_TOKEN, None, None, None, None, None, None, None, None, None,
    )
    .await
    .expect("ok");
  assert_eq!(sessions.len(), 1);
  assert_eq!(sessions[0].name, "sess-1");
}

#[tokio::test]
async fn cfs_post_session_posts_to_v3_sessions() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/cfs/v3/sessions"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"name": "new-session"})))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "name": "new-session",
      "debug_on_failure": false
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let req =
    manta_backend_dispatcher::types::cfs::session::CfsSessionPostRequest {
      name: "new-session".to_string(),
      debug_on_failure: false,
      ..Default::default()
    };
  let created = csm.post_session(TEST_TOKEN, &req).await.expect("ok");
  assert_eq!(created.name, "new-session");
}

// ---------- ApplySessionTrait ----------

// (no smoke test — the trait's `i_apply_session` is a thick orchestration
// that calls multiple endpoints and is exercised through the SAT-file
// admin workflow tests under `commands-admin`.)

// ---------- AuthenticationTrait ----------

#[tokio::test]
async fn auth_validate_api_token_hits_cfs_healthz() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/cfs/healthz"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  csm.validate_api_token(TEST_TOKEN).await.expect("ok");
}

#[tokio::test]
async fn auth_validate_api_token_returns_err_on_401() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/cfs/healthz"))
    .respond_with(ResponseTemplate::new(401))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  csm
    .validate_api_token(TEST_TOKEN)
    .await
    .expect_err("401 should map to Err");
}

// ---------- HardwareInventory::get_inventory_hardware ----------

#[tokio::test]
async fn hw_inventory_get_hits_hardware_endpoint() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/Inventory/Hardware"))
    .and(bearer_token(TEST_TOKEN))
    // NOTE: `NodeSummary::try_from_csm_value` requires `ID` *and*
    // `Type`, plus the four per-category arrays (`Processors`,
    // `Memory`, `NodeAccels`, `NodeHsnNics`) — empty arrays are fine
    // but they must be present. Missing `Type` produces a confusing
    // `HsmInventoryShape("required field 'Type' is missing")` error.
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "Nodes": [{
        "ID": "x1000c0s0b0n0",
        "Type": "Node",
        "Processors": [],
        "Memory": [],
        "NodeAccels": [],
        "NodeHsnNics": []
      }]
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let summary = csm
    .get_inventory_hardware(TEST_TOKEN, "x1000c0s0b0n0")
    .await
    .expect("ok");
  assert_eq!(summary.xname, "x1000c0s0b0n0");
}

// ---------- ComponentTrait::post_nodes ----------

#[tokio::test]
async fn component_post_nodes_posts_to_state_components() {
  // NOTE: `hsm_component_post` uses `/hsm/v2/State/Components` (no
  // `/smd/` prefix), unlike `hsm_component_get_all_nodes` which uses
  // `/smd/hsm/v2/State/Components`. The inconsistency is preserved
  // for backwards compatibility with the CSM API URL the codebase
  // has shipped against.
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/hsm/v2/State/Components"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({
      "Components": [{"ID": "x1000c0s0b0n0"}]
    })))
    .respond_with(ResponseTemplate::new(204))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  let arr = manta_backend_dispatcher::types::ComponentArrayPostArray {
    components: vec![manta_backend_dispatcher::types::ComponentCreate {
      id: "x1000c0s0b0n0".to_string(),
      state: "Ready".to_string(),
      flag: None,
      enabled: None,
      software_status: None,
      role: None,
      sub_role: None,
      nid: None,
      subtype: None,
      net_type: None,
      arch: None,
      class: None,
    }],
    force: Some(false),
  };
  csm.post_nodes(TEST_TOKEN, arr).await.expect("ok");
}

// ---------- GroupTrait::add_members_to_group ----------

#[tokio::test]
async fn group_add_members_to_group_does_get_then_post() {
  let server = MockServer::start().await;
  // hsm::group::utils::add_member does GET /smd/hsm/v2/groups?group=zinal
  // first to verify the group exists, then POST .../members with the
  // new id.
  Mock::given(method("GET"))
    .and(path("/smd/hsm/v2/groups"))
    .and(query_param("group", "zinal"))
    .and(bearer_token(TEST_TOKEN))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
      "label": "zinal",
      "members": {"ids": []}
    }])))
    .expect(1)
    .mount(&server)
    .await;
  Mock::given(method("POST"))
    .and(path("/smd/hsm/v2/groups/zinal/members"))
    .and(bearer_token(TEST_TOKEN))
    .and(body_partial_json(json!({"id": "x1000c0s0b0n0"})))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "code": 0, "message": "added"
    })))
    .expect(1)
    .mount(&server)
    .await;

  let csm = make_csm(&server.uri());
  // NOTE: don't assert on the returned member list. The underlying
  // `hsm::group::utils::add_member` calls
  // `group.get_members().push(new_member);` but `get_members` returns
  // a fresh `Vec<String>` by value (`src/hsm/group/types.rs`), so the
  // push targets a throwaway and the returned list is always
  // whatever the GET returned. This test pins the *wire* contract
  // (both endpoints fire, in order, with the right bodies) and lets
  // `.expect(1)` catch a regression. Fixing `add_member` itself is a
  // separate change.
  let _ = csm
    .add_members_to_group(TEST_TOKEN, "zinal", &["x1000c0s0b0n0"])
    .await
    .expect("ok");
}

// ---------- ComponentEthernetInterfaceTrait stubs (no network) ----------

#[tokio::test]
async fn cei_get_all_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let err = csm
    .get_all_component_ethernet_interfaces(TEST_TOKEN)
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}

#[tokio::test]
async fn cei_get_one_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let err = csm
    .get_component_ethernet_interface(TEST_TOKEN, "abc-1234")
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}

// ---------- RedfishEndpointTrait stubs (no network) ----------

#[tokio::test]
async fn redfish_add_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let endpoint =
    manta_backend_dispatcher::types::hsm::inventory::RedfishEndpointArray {
      redfish_endpoints: None,
    };
  let err = csm
    .add_redfish_endpoint(TEST_TOKEN, &endpoint)
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}

#[tokio::test]
async fn redfish_update_returns_not_implemented_error() {
  let server = MockServer::start().await;
  let csm = make_csm(&server.uri());
  let endpoint =
    manta_backend_dispatcher::types::hsm::inventory::RedfishEndpoint {
      id: "x1000c0s0b0".to_string(),
      r#type: None,
      name: None,
      hostname: None,
      domain: None,
      fqdn: None,
      enabled: None,
      uuid: None,
      user: None,
      password: None,
      use_ssdp: None,
      mac_required: None,
      mac_addr: None,
      ip_address: None,
      rediscover_on_update: None,
      template_id: None,
      discovery_info: None,
    };
  let err = csm
    .update_redfish_endpoint(TEST_TOKEN, &endpoint)
    .await
    .expect_err("stub returns Err");
  assert!(
    err.to_string().contains("not implemented"),
    "expected 'not implemented' in {err}"
  );
}
