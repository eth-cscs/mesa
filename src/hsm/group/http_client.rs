use serde_json::Value;

use crate::{
    error::Error,
    hsm::group::types::{Group, Member, Members},
};

use super::hacks::filter_system_hsm_groups;

/// Get list of HSM group using --> shttps://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doGroupsGet/
pub async fn get_raw(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    group_name_opt: Option<&String>,
) -> Result<reqwest::Response, Error> {
    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

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

    let api_url: String = if let Some(group_name) = group_name_opt {
        shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + group_name
    } else {
        shasta_base_url.to_owned() + "/smd/hsm/v2/groups"
    };

    client
        .get(api_url)
        .bearer_auth(shasta_token)
        .send()
        .await
        .map_err(|error| Error::NetError(error))
}

/// Gets list of HSM groups from CSM api. It also does a hack where the list returned by
/// CSM API gets shrinked by removing the CSM wide HSM groups like `alps`, `alpsm`,
/// `alpsb`, etc
pub async fn get(
    auth_token: &str,
    base_url: &str,
    root_cert: &[u8],
    label_vec_opt: Option<&[&str]>,
    tag_vec_opt: Option<&[&str]>,
) -> Result<Vec<Group>, Error> {
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

    let api_url: String = format!("{}/{}", base_url, "smd/hsm/v2/groups");

    let mut query = Vec::new();

    if let Some(label_vec) = label_vec_opt {
        for label in label_vec {
            query.push(("label", label));
        }
    }
    if let Some(tag_vec) = tag_vec_opt {
        for tag in tag_vec {
            query.push(("tag", tag));
        }
    }

    let response = client
        .get(api_url)
        .query(query.as_slice())
        .bearer_auth(auth_token)
        .send()
        .await?;

    if let Err(e) = response.error_for_status_ref() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error);
            }
            _ => {
                let error_payload = response.text().await?;
                let error = Error::Message(error_payload);
                return Err(error);
            }
        }
    }

    response
        .json()
        .await
        .map_err(|error| Error::NetError(error))
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Group>, Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None, None).await
}

/// Get list of HSM groups using --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doGroupsGet/
/// NOTE: this returns all HSM groups which name contains hsm_groupu_name param value
/// FIXME: change `hsm_group_name_opt` type from `Option<&String>` to Option<`&str`>
pub async fn get_hsm_group_vec(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_opt: Option<&String>,
) -> Result<Vec<Group>, Error> {
    let json_response = get_all(shasta_token, shasta_base_url, shasta_root_cert).await?;

    let mut hsm_groups: Vec<Group> = Vec::new();

    if let Some(hsm_group_name) = hsm_group_name_opt {
        for hsm_group in json_response {
            if hsm_group.label.contains(hsm_group_name) {
                hsm_groups.push(hsm_group.clone());
            }
        }
    }

    Ok(hsm_groups)
}

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    group: Group,
) -> Result<String, Error> {
    log::info!("Add/Create HSM group");
    log::debug!("Add HSM group payload:\n{:#?}", group);

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

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

    let api_url: String = shasta_base_url.to_owned() + "/smd/hsm/v2/groups";

    let response = client
        .post(api_url)
        .bearer_auth(shasta_token)
        .json(&group)
        .send()
        .await?;

    log::debug!("Response:\n{:#?}", response);

    if let Err(e) = response.error_for_status_ref() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error);
            }
            _ => {
                let error_payload = response.json().await?;
                let error = Error::Message(error_payload);
                return Err(error);
            }
        }
    }

    response
        .text()
        .await
        .map_err(|error| Error::NetError(error))
}

/// https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/smd.md#post-groups
pub async fn create_new_group(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_opt: &str, // label in HSM
    xnames: &[String],
    exclusive: &str,
    description: &str,
    tags: &[String],
) -> Result<Group, Error> {
    // Example body to create a new group:
    // {
    //   "label": "blue",
    //   "description": "This is the blue group",
    //   "tags": [
    //     "optional_tag1",
    //     "optional_tag2"
    //   ],
    //   "exclusiveGroup": "optional_excl_group",
    //   "members": {
    //     "ids": [
    //       "x1c0s1b0n0",
    //       "x1c0s1b0n1",
    //       "x1c0s2b0n0",
    //       "x1c0s2b0n1"
    //     ]
    //   }
    // }
    // Describe the JSON object

    // Create the variables that represent our JSON object
    let myxnames = Members {
        ids: Some(xnames.to_owned()),
    };

    let group = Group {
        label: hsm_group_name_opt.to_owned(),
        description: Option::from(description.to_string().clone()),
        tags: Option::from(tags.to_owned()),
        exclusive_group: Option::from(exclusive.to_string().clone()),
        members: Some(myxnames),
    };

    log::debug!("{:#?}", &group);

    let add_group_rslt = post(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        group.clone(),
    )
    .await;

    log::info!("Group created: {:?}", add_group_rslt);

    Ok(group)
}

pub async fn delete_group(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &String, // label in HSM
) -> Result<Value, Error> {
    log::info!("Delete HSM group '{}'", hsm_group_name);

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    let client = if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let url_api = shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + &hsm_group_name;

    let response = client
        .delete(url_api)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        response
            .json()
            .await
            .map_err(|error| Error::NetError(error))
    } else {
        let payload = response
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}

pub async fn post_member(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &str,
    member: Member,
) -> Result<Value, Error> {
    log::info!("Add members {}/{:?}", hsm_group_name, member);
    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

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

    let api_url: String = format!(
        "{}/hsm/v2/groups/{}/members",
        shasta_base_url, hsm_group_name
    );

    let response = client
        .post(api_url)
        .bearer_auth(shasta_token)
        .json(&member)
        .send()
        .await?;

    if let Err(e) = response.error_for_status_ref() {
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                let error_payload = response.text().await?;
                let error = Error::RequestError {
                    response: e,
                    payload: error_payload,
                };
                return Err(error);
            }
            _ => {
                let error_payload = response.json::<Value>().await?;
                let error = Error::CsmError(error_payload);
                return Err(error);
            }
        }
    }

    response
        .json()
        .await
        .map_err(|e| Error::Message(e.to_string()))
}

pub async fn delete_member(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &str,
    member_id: &str,
) -> Result<(), Error> {
    log::info!("Delete member {}/{}", hsm_group_name, member_id);
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

    let api_url: String = shasta_base_url.to_owned()
        + "/smd/hsm/v2/groups/"
        + hsm_group_name
        + "/members/"
        + member_id;

    let response = client
        .delete(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await
        .map_err(|error| Error::NetError(error))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let payload = response
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;
        Err(Error::Message(payload))
    }
}
