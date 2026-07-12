use aws_config::SdkConfig;
use aws_sdk_s3::config::Credentials;
use aws_smithy_types::timeout::TimeoutConfig;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use aws_sdk_s3::{Client, primitives::ByteStream};

use crate::error::Error;

/// Per-S3-operation deadline. The CSM reqwest client uses
/// [`crate::common::http::HTTP_REQUEST_TIMEOUT`] (15 min), but the AWS
/// SDK uses a separate transport that doesn't inherit it. A multi-GB
/// rootfs upload over a slow link can legitimately take well over 15
/// minutes, so we apply a much higher per-operation cap here (60 min)
/// and let it fail rather than letting it sit forever — the AWS SDK
/// default is "no timeout", which would block an embedder
/// indefinitely on a hung peer.
const S3_OPERATION_TIMEOUT: Duration = Duration::from_hours(1);

/// Extract the four interesting fields from a CSM STS response.
fn parse_sts_credentials(
  sts_value: &Value,
) -> Result<(Credentials, String), Error> {
  fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, Error> {
    value
      .pointer(&format!("/Credentials/{name}"))
      .and_then(Value::as_str)
      .ok_or_else(|| Error::S3Transport(format!("Missing {name} in STS response")))
  }
  let credentials = Credentials::new(
    field(sts_value, "AccessKeyId")?,
    field(sts_value, "SecretAccessKey")?,
    Some(field(sts_value, "SessionToken")?.to_string()),
    None,
    "csm-sts",
  );
  let endpoint_url = field(sts_value, "EndpointURL")?.to_string();
  Ok((credentials, endpoint_url))
}

/// Fetch an AWS STS token for the CSM-backing S3 store and return the
/// raw STS JSON response.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn s3_auth(
  shasta_token: &str,
  shasta_base_url: &str,
  shasta_root_cert: &[u8],
) -> Result<Value, Error> {
  // STS
  let client = crate::common::http::build_client(shasta_root_cert)?;

  let api_url = shasta_base_url.to_owned() + "/sts/token";

  let resp = client
    .put(api_url)
    .bearer_auth(shasta_token)
    .send()
    .await?
    .error_for_status()
    .map_err(|e| {
      Error::S3Transport(format!(
        "ERROR - could not authenticate to S3 server. Reason:\n{e}"
      ))
    })?;

  let sts_value = resp.json::<serde_json::Value>().await?;

  // NOTE: never log `sts_value` — it carries AccessKeyId, SecretAccessKey
  // and SessionToken. The "retrieved" marker is intentionally payload-free.
  log::debug!("STS token retrieved");

  Ok(sts_value)
}

async fn setup_client(sts_value: &Value) -> Result<Client, Error> {
  let (credentials, endpoint_url) = parse_sts_credentials(sts_value)?;

  let region_provider =
    aws_config::meta::region::RegionProviderChain::default_provider()
      .or_else("us-east-1");
  let app_name = aws_config::AppName::new("manta")
    .map_err(|e| Error::S3Transport(format!("Error setting app name: {e}")))?;
  let timeout_config = TimeoutConfig::builder()
    .operation_timeout(S3_OPERATION_TIMEOUT)
    .build();

  let config: SdkConfig = aws_config::from_env()
    .region(region_provider)
    .endpoint_url(endpoint_url)
    .app_name(app_name)
    .credentials_provider(credentials)
    .timeout_config(timeout_config)
    .load()
    .await;

  Ok(aws_sdk_s3::Client::from_conf(
    aws_sdk_s3::Client::new(&config)
      .config()
      .to_builder()
      .force_path_style(true)
      .build(),
  ))
}
/// Gets the size of a given object in S3
/// path of the object: <s3://bucket/key>
/// returns i64 or error
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn s3_get_object_size(
  sts_value: &Value,
  key: &str,
  bucket: &str,
) -> Result<i64, Error> {
  let client = setup_client(sts_value).await?;

  match client.get_object().bucket(bucket).key(key).send().await {
    Ok(object) => Ok(object.content_length().ok_or_else(|| {
      Error::S3Transport("Error, content length not found".to_string())
    })?),
    Err(e) => Err(Error::S3Transport(format!(
      "Error, unable to get object from s3. Error msg: {e}"
    ))),
  }
}

/// Download an object from S3 to a local directory.
///
/// Streams the object body to disk, logging byte progress. Returns the
/// full path of the downloaded file.
///
/// # Arguments
///
/// - `sts_value` — temporary S3 credentials obtained from STS via
///   `s3_auth()`.
/// - `object_path` — path within `bucket`, e.g.
///   `392o1h-1-234-w1/manifest.json`.
/// - `bucket` — bucket containing the object.
/// - `destination_path` — local directory to write into; the file is
///   placed at `destination_path/<basename(object_path)>`. Created if
///   missing.
///
/// # Errors
///
/// Returns [`Error`] if the local directory or file cannot be created,
/// or if the S3 GET fails.
pub async fn s3_download_object(
  sts_value: &Value,
  object_path: &str,
  bucket: &str,
  destination_path: &str,
) -> Result<String, Error> {
  let client = setup_client(sts_value).await?;

  let filename = Path::new(object_path).file_name().ok_or_else(|| {
    Error::S3Transport(format!(
      "Error getting filename from S3 object path: {object_path}"
    ))
  })?;

  let file_path = Path::new(destination_path).join(filename);
  log::debug!("Create directory '{destination_path}'");

  std::fs::create_dir_all(destination_path).map_err(|e| {
    Error::S3Transport(format!(
      "Error creating directory {destination_path}: {e}"
    ))
  })?;

  log::debug!("Created directory '{destination_path}' successfully");

  let mut file = File::create(&file_path).map_err(|e| {
    Error::S3Transport(format!(
      "Error creating file {}: {}",
      &file_path.to_string_lossy(),
      e
    ))
  })?;

  log::debug!(
    "Created file '{}' successfully",
    &file_path.to_string_lossy()
  );

  let mut object = client
    .get_object()
    .bucket(bucket)
    .key(object_path)
    .send()
    .await
    .map_err(|e| {
      Error::S3Transport(format!(
        "ERROR - could not download S3 object.\nReason:\n{e}",
      ))
    })?;

  let bar_size = object.content_length().ok_or_else(|| {
    Error::S3Transport("could not get S3 object size.".to_string())
  })?;
  let total = bar_size as u64;
  let mut downloaded: u64 = 0;

  while let Some(bytes) = object.body.try_next().await.map_err(|e| {
    Error::S3Transport(format!(
      "ERROR - Could not finish s3 object download.\nReason:\n{e}"
    ))
  })? {
    let bytes = file.write(&bytes)?;
    downloaded += bytes as u64;
    log::info!("downloaded {downloaded}/{total} bytes");
  }

  Ok(file_path.to_string_lossy().to_string())
}

/// Upload a local file to S3 in a single request.
///
/// Returns the `ETag` of the uploaded object.
///
/// # Arguments
///
/// - `sts_value` — temporary S3 credentials obtained from STS via
///   `s3_auth()`.
/// - `object_path` — path within `bucket` to upload to.
/// - `bucket` — destination bucket.
/// - `file_path` — local file to upload.
///
/// For large files prefer [`s3_multipart_upload_object`].
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn s3_upload_object(
  sts_value: &Value,
  object_path: &str,
  bucket: &str,
  file_path: &str,
) -> Result<String, Error> {
  let client = setup_client(sts_value).await?;

  let body = ByteStream::from_path(Path::new(&file_path)).await?;

  let put_s3_object = client
    .put_object()
    .bucket(bucket)
    .key(object_path)
    .body(body)
    .send()
    .await
    .map_err(|e| {
      Error::S3Transport(format!(
        "ERROR - could not upload S3 object.\nReason:\n{e}"
      ))
    })?;

  put_s3_object.e_tag.ok_or_else(|| {
    Error::S3Transport("could not get ETag from upload.".to_string())
  })
}

/// Delete an object from S3.
///
/// # Arguments
///
/// - `sts_value` — temporary S3 credentials obtained from STS via
///   `s3_auth()`.
/// - `object_path` — path within `bucket` to delete.
/// - `bucket` — source bucket.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn s3_remove_object(
  sts_value: &Value,
  object_path: &str,
  bucket: &str,
) -> Result<String, Error> {
  let client = setup_client(sts_value).await?;

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
    Err(error) => Err(Error::S3Transport(format!(
      "Error cleaning file {}: {}",
      &object_path, error
    ))),
  }
}

/// Upload a local file to S3 using the multipart-upload protocol.
///
/// Splits `file_path` into chunks and uploads them, logging byte progress.
/// Use this for files that exceed the single-PUT limit; for small files
/// [`s3_upload_object`] is simpler.
///
/// # Arguments
///
/// - `sts_value` — temporary S3 credentials obtained from STS via
///   `s3_auth()`.
/// - `object_path` — path within `bucket` to upload to.
/// - `bucket` — destination bucket.
/// - `file_path` — local file to upload.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn s3_multipart_upload_object(
  sts_value: &Value,
  object_path: &str,
  bucket: &str,
  file_path: &str,
) -> Result<String, Error> {
  use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
  use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
  use aws_smithy_types::byte_stream::Length;

  //In bytes, minimum chunk size of 5MB. Increase CHUNK_SIZE to send larger chunks.
  const CHUNK_SIZE: u64 = 1024 * 1024 * 5;
  const MAX_CHUNKS: u64 = 10000;

  let client = setup_client(sts_value).await?;

  // create multipart upload
  let multipart_upload_res: CreateMultipartUploadOutput = client
    .create_multipart_upload()
    .bucket(bucket)
    .key(object_path)
    .send()
    .await
    .map_err(|e| {
      Error::S3Transport(format!(
        "ERROR - Could not create multipart object.\nReason:\n{e}"
      ))
    })?;

  let upload_id = multipart_upload_res.upload_id().ok_or_else(|| {
    Error::S3Transport("could not get upload ID.".to_string())
  })?;

  // Get details of the upload, this is needed because multipart uploads
  // are tricky and have a minimum chunk size of 5MB
  let path = Path::new(&file_path);
  let file_size = std::fs::metadata(path)
    .map_err(|e| {
      Error::S3Transport(format!(
        "ERROR - Could not get file size from '{file_path}'.\nReason\n{e}"
      ))
    })?
    .len();

  let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
  let mut size_of_last_chunk = file_size % CHUNK_SIZE;
  if size_of_last_chunk == 0 {
    size_of_last_chunk = CHUNK_SIZE;
    chunk_count -= 1;
  }

  let mut uploaded: u64 = 0;

  if file_size == 0 {
    return Err(Error::S3Transport("Bad file size.".to_string()));
  }
  if chunk_count > MAX_CHUNKS {
    return Err(Error::S3Transport(
      "Too many chunks! Try increasing your chunk size.".to_string(),
    ));
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
      .map_err(|e| {
        Error::S3Transport(format!(
          "ERROR - Could not read file '{}'.\nReason:\n{}",
          path.display(),
          e
        ))
      })?;

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
      .await
      .map_err(|e| {
        Error::S3Transport(format!(
          "ERROR - could not upload to S3.\nReason:\n{e}"
        ))
      })?;

    upload_parts.push(
      CompletedPart::builder()
        .e_tag(upload_part_res.e_tag.ok_or_else(|| {
          Error::S3Transport(
            "could not get ETag from upload part.".to_string(),
          )
        })?)
        .part_number(part_number)
        .build(),
    );
    uploaded += this_chunk;
    log::info!("uploaded {uploaded}/{file_size} bytes");
  }
  // complete the multipart upload
  let completed_multipart_upload: CompletedMultipartUpload =
    CompletedMultipartUpload::builder()
      .set_parts(Some(upload_parts))
      .build();

  let complete_multipart_upload_res = client
    .complete_multipart_upload()
    .bucket(bucket)
    .key(object_path)
    .multipart_upload(completed_multipart_upload)
    .upload_id(upload_id)
    .send()
    .await
    .map_err(|e| {
      Error::S3Transport(format!("ERROR - could not upload to S3.\nReason:\n{e}"))
    })?;

  complete_multipart_upload_res.e_tag.ok_or_else(|| {
    Error::S3Transport("could not get ETag from upload.".to_string())
  })
}
