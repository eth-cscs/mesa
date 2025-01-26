use serde_json::Value;

use crate::{error::Error, hsm::component::types::Component};

use super::types::{
    ComponentArray, ComponentArrayPostArray, ComponentArrayPostByNidQuery, ComponentArrayPostQuery,
    ComponentPut,
};

pub async fn get_all(
    base_url: &str,
    root_cert: &[u8],
    auth_token: &str,
    nid_only: Option<&str>,
) -> Result<ComponentArray, Error> {
    get(
        base_url, root_cert, auth_token, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None, nid_only,
    )
    .await
}

pub async fn get_all_nodes(
    base_url: &str,
    root_cert: &[u8],
    auth_token: &str,
    nid_only: Option<&str>,
) -> Result<ComponentArray, Error> {
    get(
        base_url,
        root_cert,
        auth_token,
        None,
        Some("Node"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        nid_only,
    )
    .await
}

/// Get all components
/// NOTE: nid is a comma separated list of NIDs like "1,2,3". Value "nid0001,nid0002,nid0003" is not
/// valid values
pub async fn get(
    base_url: &str,
    root_cert: &[u8],
    auth_token: &str,
    id: Option<&str>,
    r#type: Option<&str>,
    state: Option<&str>,
    flag: Option<&str>,
    role: Option<&str>,
    subrole: Option<&str>,
    enabled: Option<&str>,
    software_status: Option<&str>,
    subtype: Option<&str>,
    arch: Option<&str>,
    class: Option<&str>,
    nid: Option<&str>,
    nid_start: Option<&str>,
    nid_end: Option<&str>,
    partition: Option<&str>,
    group: Option<&str>,
    state_only: Option<&str>,
    flag_only: Option<&str>,
    role_only: Option<&str>,
    nid_only: Option<&str>,
) -> Result<ComponentArray, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    // Create query parameters
    // NID query params
    let mut nid_vec_query = nid.map(|nids| {
        nids.split(",")
            .map(|nid| ("nid", Some(nid)))
            .collect::<Vec<(&str, Option<&str>)>>()
    });

    // All other query params
    let mut query_params = vec![
        ("id", id),
        ("type", r#type),
        ("state", state),
        ("flag", flag),
        ("role", role),
        ("subrole", subrole),
        ("enabled", enabled),
        ("softwarestatus", software_status),
        ("subtype", subtype),
        ("arch", arch),
        ("class", class),
        ("nidstart", nid_start),
        ("nidend", nid_end),
        ("partition", partition),
        ("group", group),
        ("stateonly", state_only),
        ("flagonly", flag_only),
        ("roleonly", role_only),
        ("nidonly", nid_only),
    ];

    if let Some(mut nid_vec_query) = nid_vec_query.take() {
        query_params.append(&mut nid_vec_query);
    }

    let api_url: String = format!("{}/{}", base_url, "smd/hsm/v2/State/Components");

    let response = client
        .get(api_url)
        .query(&query_params)
        .bearer_auth(auth_token)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                let error_payload = response.text().await?;
                let error = Error::Message(error_payload);
                return Err(error);

                // return Err(Error::Message(response.text().await?));
            }
            _ => {
                let error_payload = response.text().await?;
                let error = Error::Message(error_payload);
                return Err(error);
                // return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json::<ComponentArray>()
        .await
        .map_err(|e| Error::NetError(e))
}

pub async fn get_one(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    xname: &str,
) -> Result<Component, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = format!("{}/{}/{}", base_url, "hsm/v2/State/Components", xname);

    let response = client.get(api_url).bearer_auth(auth_token).send().await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn post(
    auth_token: &str,
    base_url: &str,
    root_cert: &[u8],
    component: ComponentArrayPostArray,
    // ) -> Result<ComponentArray, Error> {
) -> Result<(), Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = base_url.to_owned() + "/hsm/v2/State/Components";

    let response = client
        .post(api_url)
        .bearer_auth(auth_token)
        .json(&component)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    /* response
    .json()
    .await
    .map_err(|error| Error::NetError(error)) */

    Ok(())
}

pub async fn post_query(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    component: ComponentArrayPostQuery,
) -> Result<ComponentArray, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = base_url.to_owned() + "/hsm/v2/State/Components";

    let response = client
        .post(api_url)
        .bearer_auth(auth_token)
        .json(&component)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn post_bynid_query(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    component: ComponentArrayPostByNidQuery,
) -> Result<ComponentArray, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = base_url.to_owned() + "/hsm/v2/State/Components/ByNID/Query";

    let response = client
        .post(api_url)
        .bearer_auth(auth_token)
        .json(&component)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn put(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    xname: &str,
    component: ComponentPut,
) -> Result<(), Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = format!("{}/{}/{}", base_url, "hsm/v2/State/Components/", xname);

    let response = client
        .put(api_url)
        .bearer_auth(auth_token)
        .json(&component)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn delete_one(
    base_url: &str,
    auth_token: &str,
    root_cert: &[u8],
    xname: &str,
) -> Result<Value, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = format!("{}/{}/{}", base_url, "hsm/v2/State/Components", xname);

    let response = client
        .delete(api_url)
        .bearer_auth(auth_token)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn delete(base_url: &str, auth_token: &str, root_cert: &[u8]) -> Result<Value, Error> {
    let client_builder =
        reqwest::Client::builder().add_root_certificate(reqwest::Certificate::from_pem(root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = format!("{}/{}", base_url, "hsm/v2/State/Componnets");

    let response = client
        .delete(api_url)
        .bearer_auth(auth_token)
        .send()
        .await?;

    if !response.status().is_success() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                /* let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error); */

                return Err(Error::Message(response.text().await?));
            }
            _ => {
                /* let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error); */
                return Err(Error::CsmError(response.json::<Value>().await?));
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}
