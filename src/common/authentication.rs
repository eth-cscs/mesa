use directories::ProjectDirs;
use serde_json::Value;

use dialoguer::{Input, Password};
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

use termion::color;

use crate::error::Error;

/// docs --> https://cray-hpe.github.io/docs-csm/en-12/operations/security_and_authentication/api_authorization/
///      --> https://cray-hpe.github.io/docs-csm/en-12/operations/security_and_authentication/retrieve_an_authentication_token/
pub async fn get_api_token(
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    keycloak_base_url: &str,
    site_name: &str,
) -> Result<String, Error> {
    let mut shasta_token: String;

    // Look for authentication token in environment variable
    for (env, value) in std::env::vars() {
        if env.eq_ignore_ascii_case("MANTA_CSM_TOKEN") {
            log::info!(
                "Looking for CSM authentication token in envonment variable 'MANTA_CSM_TOKEN'"
            );

            shasta_token = value;

            match test_client_api(shasta_base_url, &shasta_token, shasta_root_cert).await {
                Ok(_) => return Ok(shasta_token),
                Err(_) => return Err(Error::Message("Authentication unsucessful".to_string())),
            }
        }
    }

    // Look for authentication token in fielsystem
    log::info!("Looking for CSM authentication token in filesystem file");

    let mut file;

    let project_dirs = ProjectDirs::from(
        "local", /*qualifier*/
        "cscs",  /*organization*/
        "manta", /*application*/
    );

    let mut path = PathBuf::from(project_dirs.unwrap().cache_dir());

    let mut attempts = 0;

    create_dir_all(&path)?;

    path.push(site_name.to_string() + "_auth"); // ~/.cache/manta/<site name>_http is the file containing the Shasta authentication
                                                // token
    log::debug!("Cache file: {:?}", path);

    shasta_token = if path.exists() {
        get_token_from_local_file(path.as_os_str()).unwrap()
    } else {
        String::new()
    };

    while !test_client_api(shasta_base_url, &shasta_token, shasta_root_cert)
        .await
        .unwrap()
        && attempts < 3
    {
        println!(
            "Please type your {}Keycloak credentials{}",
            color::Fg(color::Green),
            color::Fg(color::Reset)
        );
        let username: String = Input::new().with_prompt("username").interact_text()?;
        let password = Password::new().with_prompt("password").interact()?;

        match get_token_from_shasta_endpoint(
            keycloak_base_url,
            shasta_root_cert,
            &username,
            &password,
        )
        .await
        {
            Ok(shasta_token_aux) => {
                log::debug!("Shasta token received");
                file = File::create(&path).expect("Error encountered while creating file!");
                file.write_all(shasta_token_aux.as_bytes())
                    .expect("Error while writing to file");
                shasta_token = get_token_from_local_file(path.as_os_str()).unwrap();
            }
            Err(_) => {
                eprintln!("Failed in getting token from Shasta API");
            }
        }

        attempts += 1;
    }

    if attempts < 3 {
        shasta_token = get_token_from_local_file(path.as_os_str()).unwrap();
        Ok(shasta_token)
    } else {
        Err(Error::Message("Authentication unsucessful".to_string())) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}

pub fn get_token_from_local_file(path: &std::ffi::OsStr) -> Result<String, reqwest::Error> {
    let mut shasta_token = String::new();
    File::open(path)
        .unwrap()
        .read_to_string(&mut shasta_token)
        .unwrap();
    Ok(shasta_token.to_string())
}

pub async fn test_connectivity_to_backend(shasta_base_url: &str) -> bool {
    let client;

    let client_builder = reqwest::Client::builder().connect_timeout(Duration::new(3, 0));

    // Build client
    client = client_builder.build().unwrap();

    let api_url = shasta_base_url.to_owned() + "/cfs/healthz";

    log::info!("Validate Shasta token against {}", api_url);

    let resp_rslt = client.get(api_url).send().await;

    match resp_rslt {
        Ok(_) => true,
        Err(error) => !error.is_timeout(),
    }
}

pub async fn test_client_api(
    shasta_base_url: &str,
    shasta_token: &str,
    shasta_root_cert: &[u8],
) -> Result<bool, reqwest::Error> {
    let client;

    let client_builder = reqwest::Client::builder()
        .connect_timeout(Duration::new(1, 0))
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

    let api_url = shasta_base_url.to_owned() + "/cfs/healthz";

    log::info!("Validate Shasta token against {}", api_url);

    let resp_rslt = client.get(api_url).bearer_auth(shasta_token).send().await;

    match resp_rslt {
        Ok(resp) => {
            if resp.status().is_success() {
                log::info!("Shasta token is valid");
                return Ok(true);
            } else {
                let payload: Value = resp.json().await?;
                log::error!("Token is not valid - {}", payload);
                return Ok(false);
            }
        }
        Err(error) => {
            eprintln!("Error connecting to Shasta API. Reason:\n{:?}. Exit", error);
            log::debug!("Response:\n{:#?}", error);
            std::process::exit(1);
        }
    }
}

pub async fn get_token_from_shasta_endpoint(
    keycloak_base_url: &str,
    shasta_root_cert: &[u8],
    username: &str,
    password: &str,
) -> Result<String, reqwest::Error> {
    let mut params = HashMap::new();
    params.insert("grant_type", "password");
    params.insert("client_id", "shasta");
    params.insert("username", username);
    params.insert("password", password);

    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = format!(
        "{}/realms/shasta/protocol/openid-connect/token",
        keycloak_base_url
    );

    log::debug!("Request to fetch authentication token: {}", api_url);

    Ok(client
        .post(api_url)
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?["access_token"]
        .as_str()
        .unwrap()
        .to_string())
}
