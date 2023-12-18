use crate::ims::s3::{s3_auth, s3_upload_object, s3_download_object, s3_remove_object};
use directories::ProjectDirs;
use serde_json::Value;
use std::env::temp_dir;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

pub const TOKEN_VAR_NAME: &str = "MANTA_CSM_TOKEN";
pub const API_URL_VAR_NAME: &str = "MANTA_TEST_API_URL";

pub const BUCKET_NAME: &str = "boot-images";
pub const OBJECT_PATH: &str = "manta-test-2-delete/dummy.txt";
pub const SITE: &str = "alps";

/// # DOCS
///
/// TO RUN:
/// MANTA_TEST_API_URL="https://api-gw-service-nmn.local/apis" MANTA_CSM_TOKEN="whatever" SOCKS5=socks5h://127.0.0.1:1081 cargo test shasta::ims::s3_test -- --test-threads 1
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

async fn authenticate_with_s3() -> anyhow::Result<Value, Box<dyn Error>> {
    let shasta_token = std::env::var(TOKEN_VAR_NAME).unwrap();
    let shasta_base_url = std::env::var(API_URL_VAR_NAME).unwrap();

    // In a normal function this should come from manta::config_opts, but since we just want
    // to make sure the test works without any additional dependencies, I'm hardcoding it here.
    // XDG Base Directory Specification
    let project_dirs = ProjectDirs::from(
        "local", /*qualifier*/
        "cscs",  /*organization*/
        "manta", /*application*/
    );

    let mut config_path = PathBuf::from(project_dirs.unwrap().config_dir());
    config_path.push(SITE.to_string() + "_root_cert.pem");

    let mut shasta_root_cert = Vec::new();
    let root_cert_file_rslt = File::open(config_path);

    let _ = match root_cert_file_rslt {
        Ok(mut file) => file.read_to_end(&mut shasta_root_cert),
        Err(_) => {
            eprintln!("Root cert file for CSM not found. Exit");
            std::process::exit(1);
        }
    };

    s3_auth(&shasta_token, &shasta_base_url, &shasta_root_cert).await
}

#[tokio::test]
pub async fn test_1_s3_auth() {
    println!("----- TEST S3 AUTH -----");
    let _result = match authenticate_with_s3().await {
        Ok(_result) => assert!(true),
        Err(error) => assert!(
            false,
            "Error getting temporary s3 token from STS. Error returned: '{}'",
            error
        ),
    };
}

#[tokio::test]
pub async fn test_2_s3_put_object() {
    // tracing_subscriber::fmt::init();
    println!("----- TEST S3 PUT OBJECT -----");

    let bucket_name = BUCKET_NAME;
    let object_path = OBJECT_PATH;

    // create dummy file on the local filesystem
    let mut file1 = match NamedTempFile::new() {
        Ok(file1) => file1,
        Err(error) => panic!("{}", error.to_string()),
    };
    println!(
        "Temporary file created as {}",
        file1.path().display().to_string()
    );

    let mut file2 = match file1.reopen() {
        Ok(file2) => file2,
        Err(error) => panic!("{}", error.to_string()),
    };

    let text = "This is a temporary object used by Manta tests that can be deleted.";
    let _result = match file1.write_all(text.as_bytes()) {
        Ok(r) => r,
        Err(error) => panic!("{}", error.to_string()),
    };

    let mut buf = String::new();
    match file2.read_to_string(&mut buf) {
        Ok(_p) => println!("Contents of the file that will be uploaded: {}", buf),
        Err(error) => panic!("{}", error.to_string()),
    };

    // Connect and auth to S3
    let sts_value = match authenticate_with_s3().await {
        Ok(sts_value) => {
            println!("Debug - STS token:\n{:#?}", sts_value);
            sts_value
        }
        Err(error) => panic!("{}", error.to_string()),
    };

    // Upload dummy file
    let _result = match s3_upload_object(
        &sts_value,
        &object_path,
        &bucket_name,
        &file1.path().display().to_string(),
    )
    .await
    {
        Ok(_result) => {
            println!("Upload completed.");
        }
        Err(error) => assert!(false, "Error {}", error.to_string()),
    };
}

#[tokio::test]
pub async fn test_3_s3_get_object() {
    println!("----- TEST S3 GET OBJECT -----");

    let object_path = OBJECT_PATH;
    let bucket_name = BUCKET_NAME;

    let sts_value = match authenticate_with_s3().await {
        Ok(sts_value) => {
            println!("Debug - STS token:\n{:#?}", sts_value);
            sts_value
        }
        Err(error) => panic!("{}", error.to_string()),
    };

    let destination_path: String = temp_dir().join(object_path).display().to_string();
    println!("Downloading file {} to {}", &object_path, &destination_path);
    let _result =
        match s3_download_object(&sts_value, &object_path, &bucket_name, &destination_path).await {
            Ok(_result) => {
                println!("Download completed.");
            }
            Err(error) => assert!(false, "Error {}", error.to_string()),
        };
    assert!(true, "OK all files completed downloading.")
}

#[tokio::test]
pub async fn test_4_s3_remove_object() {
    println!("----- TEST S3 REMOVE OBJECT -----");

    let object_path = OBJECT_PATH;
    let bucket_name = BUCKET_NAME;

    let sts_value = match authenticate_with_s3().await {
        Ok(sts_value) => {
            println!("Debug - STS token:\n{:#?}", sts_value);
            sts_value
        }
        Err(error) => panic!("{}", error.to_string()),
    };

    println!("Removing file {}/ {}", &bucket_name, &object_path);

    let _result = match s3_remove_object(&sts_value, &object_path, &bucket_name).await {
        Ok(_result) => {
            println!("Object deletion completed.");
        }
        Err(error) => assert!(false, "Error {}", error.to_string()),
    };
    assert!(true, "OK, the file was removed successfully.")
}
