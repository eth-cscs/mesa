#[cfg(test)]
#[tokio::test]
async fn update_desired_configuration() {
    let token = "--REDACTED--";
    let shasta_root_cert = "--REDACTED--".as_bytes();

    super::utils::update_component_desired_configuration(
        token,
        "https://api.cmn.alps.cscs.ch/apis",
        shasta_root_cert,
        "x1001c1s5b1n1",
        "test!",
        true,
    )
    .await;
}
