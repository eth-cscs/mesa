use std::error::Error;

use serde_json::Value;

use super::r#struct::{SshContainer, Job};

/// Get IMS job ref --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/post_v3_job/
pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_root_archive_name: &str,
    artifact_id: &str,
    public_key_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let ssh_container = SshContainer {
        name: "jail".to_string(),
        jail: true,
    };

    let ssh_container_list = vec![ssh_container];

    let ims_job = Job {
        job_type: "customize".to_string(),
        image_root_archive_name: image_root_archive_name.to_string(),
        kernel_file_name: Some("kernel".to_string()),
        initrd_file_name: Some("initrd".to_string()),
        kernel_parameters_file_name: None,
        artifact_id: artifact_id.to_string(),
        public_key_id: public_key_id.to_string(),
        ssh_containers: Some(ssh_container_list),
        enable_debug: Some(false),
        buid_env_size: None,
    };

    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/ims/v3/jobs";

    let resp = client
        .post(api_url)
        .bearer_auth(shasta_token)
        .json(&ims_job)
        .send()
        .await?;

    if resp.status().is_success() {
        let response = &resp.text().await?;
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

/// Create IMS job ref --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/post_v3_job/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    job_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/ims/v3/jobs" + job_id;

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    if resp.status().is_success() {
        let response = &resp.text().await?;
        Ok(serde_json::from_str(response)?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

