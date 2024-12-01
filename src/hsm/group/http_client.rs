use crate::{
    error::Error,
    hsm::group::r#struct::{HsmGroup, Member, XnameId},
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
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    group_name_opt: Option<&String>,
) -> Result<Vec<HsmGroup>, Error> {
    let response = get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        group_name_opt,
    )
    .await?;

    if response.status().is_success() {
        if group_name_opt.is_some() {
            let payload = response
                .json::<HsmGroup>()
                .await
                .map_err(|error| Error::NetError(error))?;

            let hsm_group_vec_rslt = Ok(vec![payload]);

            //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
            //wide operations instead of using roles
            filter_system_hsm_groups(hsm_group_vec_rslt)
        } else {
            let hsm_group_vec_rslt = response
                .json()
                .await
                .map_err(|error| Error::NetError(error));

            //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
            //wide operations instead of using roles
            filter_system_hsm_groups(hsm_group_vec_rslt)
        }
    } else {
        let payload = response
            .text()
            .await
            .map_err(|error| Error::NetError(error))?;

        Err(Error::Message(payload))
    }
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<HsmGroup>, Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}

/// Get list of HSM groups using --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doGroupsGet/
/// NOTE: this returns all HSM groups which name contains hsm_groupu_name param value
/// FIXME: change `hsm_group_name_opt` type from `Option<&String>` to Option<`&str`>
pub async fn get_hsm_group_vec(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_opt: Option<&String>,
) -> Result<Vec<HsmGroup>, Error> {
    let json_response = get_all(shasta_token, shasta_base_url, shasta_root_cert).await?;

    let mut hsm_groups: Vec<HsmGroup> = Vec::new();

    if let Some(hsm_group_name) = hsm_group_name_opt {
        for hsm_group in json_response {
            if hsm_group.label.contains(hsm_group_name) {
                hsm_groups.push(hsm_group.clone());
            }
        }
    }

    Ok(hsm_groups)
}

pub async fn post_member(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &str,
    member_id: &str,
) -> Result<(), reqwest::Error> {
    log::info!("Add member {}/{}", hsm_group_name, member_id);
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

    let api_url: String =
        shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + hsm_group_name + "/members";

    let xname = XnameId {
        id: Some(member_id.to_owned()),
    };

    client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&xname) // make sure this is not a string!
        .send()
        .await?
        .error_for_status()?;
    // TODO Parse the output!!!
    // TODO add some debugging output

    Ok(())
}

pub async fn delete_member(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name: &str,
    member_id: &str,
) -> Result<(), reqwest::Error> {
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

    client
        .delete(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await?
        .error_for_status()?;

    // TODO Parse the output!!!
    // TODO add some debugging output
    Ok(())
}

/// https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/smd.md#post-groups
pub async fn create_new_hsm_group(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_opt: &str, // label in HSM
    xnames: &[String],
    exclusive: &str,
    description: &str,
    tags: &[String],
) -> Result<Vec<HsmGroup>, reqwest::Error> {
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
    let myxnames = Member {
        ids: Some(xnames.to_owned()),
    };

    let hsm_group_json = HsmGroup {
        label: hsm_group_name_opt.to_owned(),
        description: Option::from(description.to_string().clone()),
        tags: Option::from(tags.to_owned()),
        exclusive_group: Option::from(exclusive.to_string().clone()),
        members: Some(myxnames),
    };

    let hsm_group_json_body = match serde_json::to_string(&hsm_group_json) {
        Ok(m) => m,
        Err(_) => panic!(
            "Error parsing the JSON generated, one or more of the fields could have invalid chars."
        ),
    };

    println!("{:#?}", &hsm_group_json_body);

    let url_api = shasta_base_url.to_owned() + "/smd/hsm/v2/groups";

    client
        .post(url_api)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&hsm_group_json) // make sure this is not a string!
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

pub async fn delete_hsm_group(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_opt: &String, // label in HSM
) -> Result<String, reqwest::Error> {
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
    let url_api = shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + &hsm_group_name_opt;

    client
        .delete(url_api)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}
