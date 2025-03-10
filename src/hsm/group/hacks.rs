use crate::{common, error::Error, hsm};

use super::types::Group;

pub static PA_ADMIN: &str = "pa_admin";
pub static SYSTEM_WIDE_HSM_GROUPS: [&str; 4] = ["alps", "prealps", "alpse", "alpsb"];
pub static KEYCLOAK_ROLES_TO_IGNORE: [&str; 3] = [
    "offline_access",
    "uma_authorization",
    "default-roles-shasta",
];
pub static ROLES: [&str; 6] = [
    "Compute",
    "Service",
    "System",
    "Application",
    "Storage",
    "Management",
];
pub static SUBROLES: [&str; 8] = [
    "Worker",
    "Master",
    "Storage",
    "UAN",
    "Gateway",
    "LNETRouter",
    "Visualization",
    "UserDefined",
];

pub fn filter_system_hsm_groups(
    hsm_group_vec_rslt: Result<Vec<Group>, Error>,
) -> Result<Vec<Group>, Error> {
    //TODO: Get rid of this by making sure CSM admins don't create HSM groups for system
    //wide operations instead of using roles
    let hsm_group_vec_filtered_rslt: Result<Vec<Group>, Error> =
        hsm_group_vec_rslt.and_then(|hsm_group_vec| {
            Ok(hsm_group_vec
                .iter()
                .filter(|hsm_group| {
                    let label = hsm_group.label.as_str();
                    !SYSTEM_WIDE_HSM_GROUPS.contains(&label)
                })
                .cloned()
                .collect::<Vec<Group>>())
        });

    if let Ok([]) = hsm_group_vec_filtered_rslt.as_deref() {
        Err(Error::Message(format!(
            "HSM groups '{}' not allowed.",
            SYSTEM_WIDE_HSM_GROUPS.join(", ")
        )))
    } else {
        hsm_group_vec_filtered_rslt
    }
}

/// Removes unwanted roles thay may appear in keycloak auth/jwt token roles
pub fn filter_keycloak_roles(keycloak_roles: Vec<String>) -> Vec<String> {
    keycloak_roles
        .into_iter()
        .filter(|role| !KEYCLOAK_ROLES_TO_IGNORE.contains(&role.as_str()))
        .collect()
}

/// Removes 'system wide' group names
pub fn filter_system_hsm_group_names(hsm_group_name_vec: Vec<String>) -> Vec<String> {
    //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
    //wide operations instead of using roles

    hsm_group_name_vec
        .into_iter()
        .filter(|hsm_group_name| !SYSTEM_WIDE_HSM_GROUPS.contains(&hsm_group_name.as_str()))
        .collect()
}

pub fn filter_roles_and_subroles(hsm_group_name_vec: Vec<String>) -> Vec<String> {
    hsm_group_name_vec
        .into_iter()
        .filter(|hsm_group_name| {
            !ROLES.contains(&hsm_group_name.as_str())
                && !SUBROLES.contains(&hsm_group_name.as_str())
        })
        .collect()
}

/// Check user has access to all groups in CFS session
/// This function validates groups in CFS session against user auth token
/// Returns the list of groups in the CFS session the user does not have access to
pub fn validate_groups_auth_token(cfs_group_names: &[String], shasta_token: &str) -> Vec<String> {
    let keycloak_roles = common::jwt_ops::get_roles(shasta_token);

    validate_groups(cfs_group_names, &keycloak_roles)
}

/// Check user has access to all groups in CFS session
/// This function validates groups in CFS session against a list of groups the user supposedly has
/// access to
/// Returns the list of groups in the CFS session the user does not have access to
pub fn validate_groups(cfs_group_names: &[String], keycloak_roles: &[String]) -> Vec<String> {
    if keycloak_roles.contains(&PA_ADMIN.to_string()) {
        // Admins have access to all groups
        vec![]
    } else {
        // User is not admin. Check if groups in CFS session are in user auth token
        // Remove unwanted roles from keycloak auth token
        let groups_and_roles_in_auth_token =
            hsm::group::hacks::filter_keycloak_roles(keycloak_roles.to_vec());
        // Remove "roles" and "subroles" from auth token
        let site_wide_and_cluster_groups_in_auth_token =
            hsm::group::hacks::filter_roles_and_subroles(groups_and_roles_in_auth_token.to_vec());
        // Remove "site wide" (eg: alps, realps, alpsm, alpsb, etc.) from CFS session groups
        //TODO: Get rid of this by making sure CSM admins don't create HSM groups for system
        //wide operations instead of using roles
        let groups_in_user_auth_token =
            filter_system_hsm_group_names(site_wide_and_cluster_groups_in_auth_token);

        // Remove 'roles' and 'subroles' from CFS session groups
        let groups_without_roles_subroles =
            hsm::group::hacks::filter_roles_and_subroles(cfs_group_names.to_vec());
        // Remove 'system wide' groups from CFS session groups
        //TODO: Get rid of this by making sure CSM admins don't create HSM groups for system
        //wide operations instead of using roles
        let groups_without_system_wide = hsm::group::hacks::filter_system_hsm_group_names(
            groups_without_roles_subroles.to_vec(),
        );
        // Get list of groups in CFS session not in user auth token
        groups_without_system_wide
            .into_iter()
            .filter(|group| !groups_in_user_auth_token.contains(group))
            .collect()
    }
}
