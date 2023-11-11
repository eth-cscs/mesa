use std::cmp;
use std::path::Path;
use std::env::temp_dir;
use std::fs::File;
use std::io::{BufWriter, Write};
use rand::Rng;
// use tokio::fs::File;
// use tokio::io::{AsyncWriteExt, BufWriter};
use crate::shasta::ims::s3::s3::{s3_auth, s3_download_object};

/// # DOCS
///
/// TO RUN: TOKEN=$(cat ~/.cache/manta/http) SOCKS5=socks5h://127.0.0.1:1080 cargo test test_s3_connection -- --nocapture
///
/// ## Get buckets
///
/// ALPS-ncn-m001:~ # sudo cray artifacts buckets list
/// results = [ "admin-tools", "alc", "badger", "benji-backups", "boot-images", "config-data", "etcd-backup", "fw-update", "ims", "install-artifacts", "ncn-images", "ncn-utils", "nmd", "postgres-backup", "prs", "sat", "sds", "sls", "sma", "ssd", "ssi", "ssm", "vbis", "velero", "wlm",]
///
/// ## Test socks5 and rados gateway connectivity
///
/// $ curl -x socks5h://localhost:1080 http://rgw-vip
/// <?xml version="1.0" encoding="UTF-8"?><ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Owner><ID>anonymous</ID><DisplayName></DisplayName></Owner><Buckets></Buckets></ListAllMyBucketsResult>
///
/// ## Get temporary credentials
///
/// ref -> https://cray-hpe.github.io/docs-csm/en-13/operations/artifact_management/generate_temporary_s3_credentials/

#[tokio::test]
pub async fn test_s3_connection() {
    let shasta_token = std::env::var("TOKEN").unwrap(); // ~/.cache/manta/http # for testing purposes

    // STS
    let client_builder = reqwest::Client::builder().danger_accept_invalid_certs(true);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env).unwrap();

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build().unwrap()
    } else {
        client_builder.build().unwrap()
    };

    let api_url = "https://api-gw-service-nmn.local/apis/sts/token";

    let resp = client
        .put(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .unwrap();

    let sts_value = if resp.status().is_success() {
        resp.json::<serde_json::Value>().await.unwrap()
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await.unwrap();
        eprintln!("FAIL response: {:#?}", response);
        std::process::exit(1);
    };

    println!("-- STS Token retrieved --");
    println!("Debug - STS token:\n{:#?}", sts_value);

    // SET AUTH ENVS
    std::env::set_var(
        "AWS_SESSION_TOKEN",
        sts_value["Credentials"]["SessionToken"].as_str().unwrap(),
    );
    std::env::set_var(
        "AWS_ACCESS_KEY_ID",
        sts_value["Credentials"]["AccessKeyId"].as_str().unwrap(),
    );
    std::env::set_var(
        "AWS_SECRET_ACCESS_KEY",
        sts_value["Credentials"]["SecretAccessKey"]
            .as_str()
            .unwrap(),
    );

    // S3 STUFF
    if let Ok(socks5_env) = std::env::var("SOCKS5") {
        println!("socks5 defined: {}", socks5_env);
    } else {
        println!("socks5 NOT defined");
    }

    let mut http_connector = hyper::client::HttpConnector::new();
    http_connector.enforce_http(false);
    let socks_http_connector = hyper_socks2::SocksConnector {
        proxy_addr: "socks5h://127.0.0.1:1080".parse::<hyper::Uri>().unwrap(), // scheme is required by HttpConnector
        auth: None,
        connector: http_connector.clone(),
    };

    let smithy_connector = aws_smithy_client::hyper_ext::Adapter::builder()
        // Optionally set things like timeouts as well
        .connector_settings(
            aws_smithy_client::http_connector::ConnectorSettings::builder()
                .connect_timeout(std::time::Duration::from_secs(5))
                .build(),
        )
        .build(socks_http_connector);

    let region_provider =
        aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1");

    let config = aws_config::from_env()
        .region(region_provider)
        .http_connector(smithy_connector)
        .endpoint_url(sts_value["Credentials"]["EndpointURL"].as_str().unwrap())
        .app_name(aws_config::AppName::new("manta").unwrap())
        // .no_credentials()
        .load()
        .await;

    let client = aws_sdk_s3::Client::new(&config);

    let resp_rslt = client.list_buckets().send().await;

    match resp_rslt {
        Ok(resp) => {
            // println!("DEBUG - DATA:\n{:#?}", resp);

            // let buckets = resp.buckets().unwrap();
            let buckets = resp.buckets();

            println!("Debug - Buckets:\n{:?}", buckets);
        }
        Err(error) => eprintln!("Error: {:#?}", error),
    };

    std::process::exit(0);
}

#[tokio::test]
pub async fn test_s3_auth() {
    println!("----- TEST S3 AUTH -----");

    let shasta_token = std::env::var("MANTA_CSM_TOKEN").unwrap();
    let shasta_base_url = "https://api-gw-service-nmn.local/apis";

    let _sts_value = match s3_auth(&shasta_token, &shasta_base_url).await {
        // Ok(sts_value) => sts_value,
        Ok(_sts_value) => assert!(true),
        Err(error) => assert!(false,"Error getting temporary s3 token from STS. Error returned: '{}'", error),
    };
}
#[tokio::test]
pub async fn test_s3_get_object() {
    tracing_subscriber::fmt::init();
    println!("----- TEST S3 GET OBJECT -----");

    let shasta_token = std::env::var("MANTA_CSM_TOKEN").unwrap();
    let shasta_base_url = "https://api-gw-service-nmn.local/apis";
    let image_id = "58a205ff-d98a-46ad-a32d-87657c90814e";
    let files = ["manifest.json", "initrd"];
    let bucket_name = "boot-images";

    let sts_value = match s3_auth(&shasta_token, &shasta_base_url).await {
        // Ok(sts_value) => sts_value,
        Ok(sts_value) => {
            println!("Debug - STS token:\n{:#?}", sts_value);
            sts_value
        },
        Err(error) => {
            panic!("{}", error.to_string())
        },
    };

    let destination_path:String = temp_dir().join(image_id).display().to_string();
    for file in files {
        let object_path:String = Path::new(&image_id).join(&file).display().to_string();
        println!("Downloading file {} to {}/{}", &object_path, &destination_path,&file);
        let _result = match s3_download_object(&sts_value,
                                               &object_path,
                                               &bucket_name,
                                               &destination_path).await {
            Ok(_result) => {
                println!("Download completed.");
            },
            Err(error) => assert!(false, "Error {}", error.to_string())
        };
    }

    assert!(true, "OK all files completed downloading.")
}

#[tokio::test]
pub async fn test_s3_put_object() {
    tracing_subscriber::fmt::init();
    println!("----- TEST S3 PUT OBJECT -----");
    let size = 100;
    // create dummy file on the local filesystem
    let f = File::create("/tmp/whatever.txt").unwrap();
    let mut writer = BufWriter::new(f);

    let mut rng = rand::thread_rng();
    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = cmp::min(remaining_size, buffer.len());
        let buffer=  &mut buffer[..to_write];
        rng.fill(buffer);
        writer.write(buffer).unwrap();

        remaining_size -= to_write;
    }
    // upload dummy file

    // remove dummy file from the local filesystem
}