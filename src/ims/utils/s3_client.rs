use aws_config::SdkConfig;
use hyper::client::HttpConnector;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde_json::Value;

use anyhow::Result;
use aws_sdk_s3::{primitives::ByteStream, Client};
use indicatif::{ProgressBar, ProgressStyle};

pub const BAR_FORMAT: &str = "[{elapsed_precise}] {bar:40.cyan/blue} ({bytes_per_sec}) {bytes:>7}/{total_bytes:7} {msg} [ETA {eta}]";
// Get a token for S3 and return the result
// If something breaks, return an error
pub async fn s3_auth(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Value, reqwest::Error> {
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
        .await?
        .error_for_status()?;

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
}

async fn setup_client(sts_value: &Value) -> Client {
    use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

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
        // let smithy_connector = aws_smithy_client::hyper_ext::Adapter::builder()
        //     // Optionally set things like timeouts as well
        //     .connector_settings(
        //         aws_smithy_client::http_connector::ConnectorSettings::builder()
        //             .connect_timeout(std::time::Duration::from_secs(10))
        //             .build(),
        //     )
        //     .build(socks_http_connector);
        let http_client = HyperClientBuilder::new().build(socks_http_connector);

        config = aws_config::from_env()
            .region(region_provider)
            .http_client(http_client)
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
/// Gets the size of a given object in S3
/// path of the object: s3://bucket/key
/// returns i64 or error
pub async fn s3_get_object_size(
    sts_value: &Value,
    key: &str,
    bucket: &str,
) -> Result<i64, Box<dyn Error>> {
    let client = setup_client(sts_value).await;
    match client.get_object().bucket(bucket).key(key).send().await {
        Ok(object) => Ok(object.content_length().unwrap()),
        Err(e) => panic!("Error, unable to get object size from s3. Error msg: {}", e),
    }
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
    let client = setup_client(sts_value).await;

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

    let mut object = client
        .get_object()
        .bucket(bucket)
        .key(object_path)
        .send()
        .await?;

    let bar_size = object.content_length().unwrap();
    let bar = ProgressBar::new(bar_size as u64);
    bar.set_style(ProgressStyle::with_template(BAR_FORMAT).unwrap());

    while let Some(bytes) = object.body.try_next().await? {
        let bytes = file.write(&bytes)?;
        bar.inc(bytes as u64);
    }
    bar.finish();
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
    let client = setup_client(sts_value).await;

    let body = ByteStream::from_path(Path::new(&file_path)).await;

    match client
        .put_object()
        .bucket(bucket)
        .key(object_path)
        .body(body.unwrap())
        .send()
        .await
    {
        Ok(put_object_output) => {
            log::debug!("Uploaded file '{}' successfully", &file_path);
            Ok(put_object_output.e_tag.unwrap())
        }
        Err(error) => panic!("Error uploading file {}: {}", &file_path, error),
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
    let client = setup_client(sts_value).await;

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
    }
}

/// Uploads an object to S3 using the multipart method
///
/// # Needs
/// - `sts_value` the temporary S3 token obtained from STS via `s3_auth()`
/// - `object_path` path within the bucket in S3 of the object e.g. `392o1h-1-234-w1/manifest.json`
/// - `bucket` bucket where the object will be stored
/// - `file_path` <p>path in the local filesystem where the file is located
/// # Returns
///   * String: size the object uploaded OR
///   * Box<dyn Error>: descriptive error if not possible to upload the object
pub async fn s3_multipart_upload_object(
    sts_value: &Value,
    object_path: &str,
    bucket: &str,
    file_path: &str,
) -> Result<String, Box<dyn Error>> {
    use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
    use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
    use aws_smithy_types::byte_stream::Length;

    let client = setup_client(sts_value).await;

    //In bytes, minimum chunk size of 5MB. Increase CHUNK_SIZE to send larger chunks.
    const CHUNK_SIZE: u64 = 1024 * 1024 * 5;
    const MAX_CHUNKS: u64 = 10000;

    // create multipart upload
    let multipart_upload_res: CreateMultipartUploadOutput = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(object_path)
        .send()
        .await
        .unwrap();

    let upload_id = multipart_upload_res.upload_id().unwrap();

    // Get details of the upload, this is needed because multipart uploads
    // are tricky and have a minimum chunk size of 5MB
    let path = Path::new(&file_path);
    let file_size = std::fs::metadata(path).expect("it exists I swear").len();

    let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = CHUNK_SIZE;
        chunk_count -= 1;
    }

    let bar = ProgressBar::new(file_size);
    bar.set_style(ProgressStyle::with_template(BAR_FORMAT).unwrap());

    if file_size == 0 {
        panic!("Bad file size.");
    }
    if chunk_count > MAX_CHUNKS {
        panic!("Too many chunks! Try increasing your chunk size.")
    }

    let mut upload_parts: Vec<CompletedPart> = Vec::new();

    for chunk_index in 0..chunk_count {
        let this_chunk = if chunk_count - 1 == chunk_index {
            size_of_last_chunk
        } else {
            CHUNK_SIZE
        };
        let stream = ByteStream::read_from()
            .path(path)
            .offset(chunk_index * CHUNK_SIZE)
            .length(Length::Exact(this_chunk))
            .build()
            .await
            .unwrap();
        //Chunk index needs to start at 0, but part numbers start at 1.
        let part_number = (chunk_index as i32) + 1;
        let upload_part_res = client
            .upload_part()
            .key(object_path)
            .bucket(bucket)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await?;
        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
        bar.inc(this_chunk);
    }
    // complete the multipart upload
    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    let _complete_multipart_upload_res = client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(object_path)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await
        .unwrap();

    bar.finish();

    Ok(_complete_multipart_upload_res.e_tag.clone().unwrap())
}
