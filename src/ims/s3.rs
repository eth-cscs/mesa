use aws_config::SdkConfig;
use hyper::client::HttpConnector;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde_json::Value;
use tokio_stream::StreamExt;

use anyhow::Result;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;

// Get a token for S3 and return the result
// If something breaks, return an error
pub async fn s3_auth(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Value, Box<dyn Error>> {
    // STS
    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

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

    let api_url = shasta_base_url.to_owned() + "/sts/token";

    let resp = client
        .put(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .unwrap();

    if resp.status().is_success() {
        let sts_value = resp.json::<serde_json::Value>().await.unwrap();

        log::debug!("-- STS Token retrieved --");
        log::debug!("Debug - STS token:\n{:#?}", sts_value);
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

        Ok(sts_value)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await.unwrap();
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

async fn setup_client(sts_value: &Value) -> Client {
    // Default provider fallback to us-east-1 since CSM doesn't use the concept of regions
    let region_provider =
        aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1");
    let config: SdkConfig;

    if let Ok(socks5_env) = std::env::var("SOCKS5") {
        log::debug!("SOCKS5 enabled");

        let mut http_connector: HttpConnector = hyper::client::HttpConnector::new();
        http_connector.enforce_http(false);

        let socks_http_connector = hyper_socks2::SocksConnector {
            proxy_addr: socks5_env.to_string().parse::<hyper::Uri>().unwrap(), // scheme is required by HttpConnector
            auth: None,
            connector: http_connector.clone(),
        };
        let smithy_connector = aws_smithy_client::hyper_ext::Adapter::builder()
            // Optionally set things like timeouts as well
            .connector_settings(
                aws_smithy_client::http_connector::ConnectorSettings::builder()
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .build(),
            )
            .build(socks_http_connector);

        config = aws_config::from_env()
            .region(region_provider)
            .http_connector(smithy_connector)
            .endpoint_url(sts_value["Credentials"]["EndpointURL"].as_str().unwrap())
            .app_name(aws_config::AppName::new("manta").unwrap())
            // .no_credentials()
            .load()
            .await;
    } else {
        config = aws_config::from_env()
            .region(region_provider)
            .endpoint_url(sts_value["Credentials"]["EndpointURL"].as_str().unwrap())
            .app_name(aws_config::AppName::new("manta").unwrap())
            // .no_credentials()
            .load()
            .await;
    }

    let client = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Client::new(&config)
            .config()
            .to_builder()
            .force_path_style(true)
            .build(),
    );
    client
}
/// Gets an object from S3
///
/// # Needs
/// - `sts_value` the temporary S3 token obtained from STS via `s3_auth()`
/// - `object_path` path within the bucket in S3 of the object e.g. `392o1h-1-234-w1/manifest.json`
/// - `bucket` bucket where the object is contained.
/// - `destination_path` <p>path in the local filesystem where the file will be downloaded to
///         e.g. `/tmp/my_images/392o1h-1-234-w1` the file will be downloaded then to
///             `/tmp/my_images/392o1h-1-234-w1/manifest.json`</p>
/// # Returns
///   * String: full path of the object downloaded OR
///   * Box<dyn Error>: descriptive error if not possible to download or to store the object
pub async fn s3_download_object(
    sts_value: &Value,
    object_path: &str,
    bucket: &str,
    destination_path: &str,
) -> Result<String, Box<dyn Error>> {
    let client = setup_client(&sts_value).await;

    let filename = Path::new(object_path).file_name().unwrap();
    let file_path = Path::new(destination_path).join(filename);
    log::debug!("Create directory '{}'", destination_path);

    match std::fs::create_dir_all(destination_path) {
        Ok(_) => log::debug!("Created directory '{}' successfully", destination_path),
        Err(error) => panic!("Error creating directory {}: {}", destination_path, error),
    };
    let mut file = match File::create(&file_path) {
        Ok(file) => {
            log::debug!(
                "Created file '{}' successfully",
                &file_path.to_string_lossy()
            );
            file
        }
        Err(error) => panic!(
            "Error creating file {}: {}",
            &file_path.to_string_lossy(),
            error
        ),
    };

    // --- list buckets ---
    // let resp_rslt = client.list_buckets().send().await;
    // match resp_rslt {
    //     Ok(resp) => {
    //         // println!("DEBUG - DATA:\n{:#?}", resp);
    //
    //         let buckets = resp.buckets().unwrap();
    //         // let buckets = resp.buckets();
    //
    //         println!("Debug - Buckets:\n{:?}", buckets);
    //     }
    //     Err(error) => eprintln!("Error: {:#?}", error),
    // };

    // let resp = client.list_objects_v2().bucket(bucket).send().await;
    // match resp {
    //     Ok(resp) => println!("DEBUG - DATA:\n{:#?}", resp),
    //     Err(error) => eprintln!("Error: {:#?}", error),
    // }

    // --- Download file from the specified bucket ---
    let mut object = client
        .get_object()
        .bucket(bucket)
        .key(object_path)
        .send()
        .await?;

    // // let byte_count = 0_usize;
    while let Some(bytes) = object.body.try_next().await? {
        let bytes = file.write(&bytes)?;
        // byte_count += bytes;
        log::debug!("Intermediate write of {bytes} bytes...");
    }
    //

    Ok(file_path.to_string_lossy().to_string())
}

/// Uploads an object to S3
///
/// # Needs
/// - `sts_value` the temporary S3 token obtained from STS via `s3_auth()`
/// - `object_path` path within the bucket in S3 of the object e.g. `392o1h-1-234-w1/manifest.json`
/// - `bucket` bucket where the object will be stored
/// - `file_path` <p>path in the local filesystem where the file is located
/// # Returns
///   * String: size the object uploaded OR
///   * Box<dyn Error>: descriptive error if not possible to upload the object
pub async fn s3_upload_object(
    sts_value: &Value,
    object_path: &str,
    bucket: &str,
    file_path: &str,
) -> Result<String, Box<dyn Error>> {
    let client = setup_client(&sts_value).await;

    let body = ByteStream::from_path(Path::new(&file_path)).await;

    match client
        .put_object()
        .bucket(bucket)
        .key(object_path)
        .body(body.unwrap())
        .send()
        .await
    {
        Ok(_file) => {
            log::debug!("Uploaded file '{}' successfully", &file_path);
            Ok(String::from("client"))
        }
        Err(error) => panic!("Error uploading file {}: {}", &file_path, error),
        //
    }
}

/// Removes an object from S3
///
/// # Needs
/// - `sts_value` the temporary S3 token obtained from STS via `s3_auth()`
/// - `object_path` path within the bucket in S3 of the object e.g. `392o1h-1-234-w1/manifest.json`
/// - `bucket` bucket where the object will be stored
/// # Returns
///   * String: size the object uploaded OR
///   * Box<dyn Error>: descriptive error if not possible to upload the object
pub async fn s3_remove_object(
    sts_value: &Value,
    object_path: &str,
    bucket: &str,
) -> Result<String, Box<dyn Error>> {
    let client = setup_client(&sts_value).await;

    match client
        .delete_object()
        .bucket(bucket)
        .key(object_path)
        .send()
        .await
    {
        Ok(_file) => {
            log::debug!("Cleaned file '{}' successfully", &object_path);
            Ok(String::from("client"))
        }
        Err(error) => panic!("Error cleaning file {}: {}", &object_path, error),
        //
    }
}
