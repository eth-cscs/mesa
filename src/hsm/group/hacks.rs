use crate::error::Error;

use super::types::Group;

pub fn filter_system_hsm_groups(
    hsm_group_vec_rslt: Result<Vec<Group>, Error>,
) -> Result<Vec<Group>, Error> {
    //TODO: Get rid of this by making sure CSM admins don't create HSM groups for system
    //wide operations instead of using roles
    let hsm_group_to_ignore_vec = ["alps", "prealps", "alpse", "alpsb"];
    let hsm_group_vec_filtered_rslt: Result<Vec<Group>, Error> =
        hsm_group_vec_rslt.and_then(|hsm_group_vec| {
            Ok(hsm_group_vec
                .iter()
                .filter(|hsm_group| {
                    let label = hsm_group.label.as_str();
                    !hsm_group_to_ignore_vec.contains(&label)
                })
                .cloned()
                .collect::<Vec<Group>>())
        });

    if let Ok([]) = hsm_group_vec_filtered_rslt.as_deref() {
        Err(Error::Message(
            "HSM groups 'alps, prealps, alpse, alpsb' not allowed.".to_string(),
        ))
    } else {
        hsm_group_vec_filtered_rslt
    }
}

pub fn filter_system_hsm_group_names(hsm_group_name_vec: Vec<String>) -> Vec<String> {
    //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
    //wide operations instead of using roles
    let hsm_group_to_ignore_vec = ["alps", "prealps", "alpse", "alpsb"];

    hsm_group_name_vec
        .into_iter()
        .filter(|hsm_group_name| !hsm_group_to_ignore_vec.contains(&hsm_group_name.as_str()))
        .collect()
}
