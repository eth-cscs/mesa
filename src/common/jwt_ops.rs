use crate::error::Error;
use base64::decode;
use serde_json::Value;

use crate::hsm;

/* // FIXME: replace Error to my own one
#[deprecated(
    note = "Please, avoid using this function, if you need to get the list of HSM groups available to the user, then use `mesa::common::jwt_ops::get_hsm_name_available` because this function has the hack removing system wide hsm group names like alps, aplsm, alpse, etc. If you want the preffereed username, then use `mesa::common::jwt_ops::`mesa::common::jwt_ops::get_preferred_username"
)] */
fn get_claims_from_jwt_token(token: &str) -> Result<Value, Error> {
    let base64_claims = token
        .split(' ')
        .nth(1)
        .unwrap_or(token)
        .split('.')
        .nth(1)
        .unwrap_or("JWT Token not valid");

    let claims_u8 = decode(base64_claims).map_err(|e| {
        Error::Message(format!(
            "ERROR - could not get claims in JWT token. Reason:\n{}",
            e
        ))
    })?;

    let claims_str = std::str::from_utf8(&claims_u8).map_err(|_| {
        Error::Message("ERROR - could not convert JWT claims to string".to_string())
    })?;

    serde_json::from_str::<Value>(claims_str).map_err(|_| {
        Error::Message("ERROR - could not convert JWT claims to a JSON object".to_string())
    })
}

pub fn get_name(token: &str) -> Result<String, Error> {
    let jwt_claims = get_claims_from_jwt_token(token).unwrap();

    let jwt_name = jwt_claims["name"].as_str();

    match jwt_name {
        Some(name) => Ok(name.to_string()),
        None => Err(Error::Message(
            "ERROR - claim 'name' not found in JWT auth token".to_string(),
        )),
    }
}

pub fn get_preferred_username(token: &str) -> Result<String, Error> {
    let jwt_claims = get_claims_from_jwt_token(token).unwrap();

    let jwt_preferred_username = jwt_claims["preferred_username"].as_str();

    match jwt_preferred_username {
        Some(name) => Ok(name.to_string()),
        None => Err(Error::Message(
            "ERROR - claim 'name' not found in JWT auth token".to_string(),
        )),
    }
}

/// Returns the list of available HSM groups in JWT user token. The list is filtered and system HSM
/// groups (eg alps, alpsm, alpse, etc)
pub fn get_hsm_name_available(token: &str) -> Result<Vec<String>, Error> {
    // If JWT does not have `/realm_access/roles` claim, then we will assume, user is admin
    let mut hsm_name_available_vec: Vec<String> = get_claims_from_jwt_token(token)
        .unwrap()
        .pointer("/realm_access/roles")
        .unwrap_or(&serde_json::json!([]))
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|role_value| role_value.as_str().unwrap().to_string())
        .collect();

    hsm_name_available_vec
        .retain(|role| !role.eq("offline_access") && !role.eq("uma_authorization"));

    //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
    //wide operations instead of using roles
    Ok(hsm::group::hacks::filter_system_hsm_group_names(
        hsm_name_available_vec,
    ))
}
