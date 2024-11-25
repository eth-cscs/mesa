use crate::cfs::session::csm::v3::r#struct::CfsSessionGetResponse;

#[tokio::test]
async fn test_bos_sessiontemplate_serde_json_to_struct_conversion() {
    let bos_sessiontemplate_value = serde_json::json!({
      "boot_sets": {
        "compute": {
          "etag": "44d82a32878a3abbe461c38b071c55bc",
          "kernel_parameters": "ip=dhcp quiet spire_join_token=${SPIRE_JOIN_TOKEN}",
          "node_groups": [
            "muttler"
          ],
          "path": "s3://boot-images/2105dd38-2c8e-48c5-8b3f-ca71367a977e/manifest.json",
          "rootfs_provider": "cpss3",
          "rootfs_provider_passthrough": "dvs:api-gw-service-nmn.local:300:nmn0",
          "type": "s3"
        }
      },
      "cfs": {
        "configuration": "muttler-cos-config-20221012100753"
      },
      "enable_cfs": true,
      "name": "muttler-cos-template-20221012100753"
    });

    let bos_sessiontemplate =
        serde_json::from_value::<CfsSessionGetResponse>(bos_sessiontemplate_value);

    println!("{:#?}", bos_sessiontemplate);
}
