use crate::cfs::session::http_client::v3::r#struct::CfsSessionGetResponse;

#[tokio::test]
async fn test_cfs_session_serde_json_to_struct_conversion() {
    let cfs_session_value = serde_json::json!({
      "ansible": {
        "config": "cfs-default-ansible-cfg",
        "limit": "x1005c1s2b0n0,x1005c0s3b0n0",
        "passthrough": null,
        "verbosity": 0
      },
      "configuration": {
        "limit": "",
        "name": "clariden-cos-config-2.3.110-96-3"
      },
      "name": "batcher-e5c059a8-20c1-4779-9c0b-a270ff081d63",
      "status": {
        "artifacts": [],
        "session": {
          "completionTime": "2023-10-10T08:46:34",
          "job": "cfs-298b9145-7504-4241-a985-7a2f301cdd9f",
          "startTime": "2023-10-10T08:36:40",
          "status": "complete",
          "succeeded": "true"
        }
      },
      "tags": {
        "bos_session": "d452344f-4aad-4747-bfcb-8d016b5524bc"
      },
      "target": {
        "definition": "dynamic",
        "groups": null
      }
    });

    let cfs_session = serde_json::from_value::<CfsSessionGetResponse>(cfs_session_value);

    println!("{:#?}", cfs_session);
}
