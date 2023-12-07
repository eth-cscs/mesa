use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use directories::ProjectDirs;
use serde_json::{to_string, Value};
use crate::shasta::hsm::http_client::create_new_hsm_group;

pub const TOKEN_VAR_NAME:&str = "MANTA_CSM_TOKEN";
pub const API_URL_VAR_NAME:&str = "MANTA_TEST_API_URL";

pub const SITE:&str = "alps";

#[tokio::test]
pub async fn test_1_hsm_create_new_hsm_group() {
    let shasta_token = std::env::var(crate::shasta::ims::s3_test::TOKEN_VAR_NAME).unwrap();
    let shasta_base_url = std::env::var(crate::shasta::ims::s3_test::API_URL_VAR_NAME).unwrap();

    // TODO Fix the xnames for Alps, this can cause trouble as it is
    let exclusive:bool = false; // Make sure this is false, so we can test this without impacting other HSM groups
    // the following xnames are part of HSM group "gele"
    let xnames:Vec<String> = vec!["x1001c7s1b0n0".to_string(),
                                  "x1001c7s1b0n1".to_string(),
                                  "x1001c7s1b1n0".to_string(),
                                  "x1001c7s1b1n1".to_string()];
    let description = "Test group created by function mesa test_1_hsm";
    let tags:Vec<String> = vec!["dummyTag1".to_string(), "dummyTag2".to_string()];
    // let tags= vec![]; // sending an empty vector works
    let hsm_group_name_opt = "manta_created_hsm".to_string();

    // In a normal function this should come from manta::config_opts, but since we just want
    // to make sure the test works without any additional dependencies, I'm hardcoding it here.
    // XDG Base Directory Specification
    let project_dirs = ProjectDirs::from(
        "local", /*qualifier*/
        "cscs",  /*organization*/
        "manta", /*application*/
    );

    let mut config_path = PathBuf::from(project_dirs.unwrap().config_dir());
    config_path.push(crate::shasta::ims::s3_test::SITE.to_string() + "_root_cert.pem");

    let mut shasta_root_cert = Vec::new();
    let root_cert_file_rslt = File::open(config_path);

    let _ = match root_cert_file_rslt {
        Ok(mut file) => file.read_to_end(&mut shasta_root_cert),
        Err(_) => {
            eprintln!("Root cert file for CSM not found. Exit");
            std::process::exit(1);
        }
    };

    let _json = match create_new_hsm_group(&shasta_token,
                         &shasta_base_url,
                         &shasta_root_cert,
                         &hsm_group_name_opt,
                         &xnames,
                         &exclusive,
                         &description,
                         &tags).await {
        Ok(_json) => assert!(true),
        Err(error) => assert!(false,"Error creating a new HSM group. Error returned: '{}'", error),
    };
}
