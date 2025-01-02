use crate::{cfs, hsm};
use std::io::{self, Write};

use super::http_client::v3::r#struct::CfsSessionGetResponse;

/// Filter CFS sessions related to a list of HSM group names, how this works is, it will
/// get the list of nodes within those HSM groups and filter all CFS sessions in the system
/// using either the HSM group names or nodes as target.
/// NOTE: Please make sure the user has access to the HSM groups he is asking for before
/// calling this function
pub async fn filter_by_hsm(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
    hsm_group_name_vec: &[String],
    limit_number_opt: Option<&u8>,
) {
    log::info!("Filter CFS sessions");
    let xname_vec: Vec<String> = hsm::group::utils::get_member_vec_from_hsm_name_vec(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec.to_vec(),
    )
    .await;

    // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
    // hsm_group.members.ids
    if !hsm_group_name_vec.is_empty() {
        cfs_session_vec.retain(|cfs_session| {
            cfs_session.get_target_hsm().is_some_and(|target_hsm_vec| {
                target_hsm_vec
                    .iter()
                    .any(|target_hsm| hsm_group_name_vec.contains(target_hsm))
            }) || cfs_session
                .get_target_xname()
                .is_some_and(|target_xname_vec| {
                    target_xname_vec
                        .iter()
                        .any(|target_xname| xname_vec.contains(target_xname))
                })
        });
    }

    // Sort CFS sessions by start time order ASC
    cfs_session_vec.sort_by(|a, b| {
        a.status
            .as_ref()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .start_time
            .as_ref()
            .unwrap()
            .cmp(
                b.status
                    .as_ref()
                    .unwrap()
                    .session
                    .as_ref()
                    .unwrap()
                    .start_time
                    .as_ref()
                    .unwrap(),
            )
    });

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        *cfs_session_vec = cfs_session_vec
            [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
            .to_vec();
    }
}

pub async fn filter_by_xname(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
    xname_vec: &[&str],
    limit_number_opt: Option<&u8>,
) {
    let hsm_group_name_from_xnames_vec: Vec<String> =
        hsm::group::utils::get_hsm_group_name_vec_from_xname_vec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            xname_vec,
        )
        .await;

    log::info!(
        "HSM groups that belongs to xnames {:?} are: {:?}",
        xname_vec,
        hsm_group_name_from_xnames_vec
    );

    // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
    // hsm_group.members.ids
    if !hsm_group_name_from_xnames_vec.is_empty() {
        cfs_session_vec.retain(|cfs_session| {
            cfs_session.get_target_hsm().is_some_and(|target_hsm_vec| {
                target_hsm_vec
                    .iter()
                    .any(|target_hsm| hsm_group_name_from_xnames_vec.contains(target_hsm))
            }) || cfs_session
                .get_target_xname()
                .is_some_and(|target_xname_vec| {
                    target_xname_vec
                        .iter()
                        .any(|target_xname| xname_vec.contains(&target_xname.as_str()))
                })
        });
    }

    // Sort CFS sessions by start time order ASC
    cfs_session_vec.sort_by(|a, b| {
        a.status
            .as_ref()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .start_time
            .as_ref()
            .unwrap()
            .cmp(
                b.status
                    .as_ref()
                    .unwrap()
                    .session
                    .as_ref()
                    .unwrap()
                    .start_time
                    .as_ref()
                    .unwrap(),
            )
    });

    if let Some(limit_number) = limit_number_opt {
        // Limiting the number of results to return to client
        *cfs_session_vec = cfs_session_vec
            [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
            .to_vec();
    }
}

/// Filter CFS sessions to the ones related to a CFS configuration
pub fn filter_by_cofiguration(
    cfs_session_vec: &mut Vec<CfsSessionGetResponse>,
    cfs_configuration_name: &str,
) {
    cfs_session_vec.retain(|cfs_session| {
        cfs_session.get_configuration_name().as_deref() == Some(cfs_configuration_name)
    });
}

/// Filter CFS sessions related to a list of HSM group names and a list of nodes and filter
/// all CFS sessions in the system using either the HSM group names or nodes as target.
/// NOTE: Please make sure the user has access to the HSM groups and nodes he is asking for before
/// calling this function
pub fn find_cfs_session_related_to_image_id(
    cfs_session_vec: &[CfsSessionGetResponse],
    image_id: &str,
) -> Option<CfsSessionGetResponse> {
    cfs_session_vec
        .iter()
        .find(|cfs_session| {
            cfs_session
                .get_first_result_id()
                .is_some_and(|result_id| result_id == image_id)
        })
        .cloned()
}

pub fn get_cfs_configuration_name(cfs_session: &CfsSessionGetResponse) -> Option<String> {
    cfs_session
        .configuration
        .as_ref()
        .unwrap()
        .name
        .as_ref()
        .cloned()
}

/// Returns a tuple like (image_id, cfs_configuration_name, target) from a list of CFS
/// sessions
pub fn get_image_id_cfs_configuration_target_tuple_vec(
    cfs_session_vec: Vec<CfsSessionGetResponse>,
) -> Vec<(String, String, Vec<String>)> {
    let mut image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
        Vec::new();

    cfs_session_vec.iter().for_each(|cfs_session| {
        let result_id: String = cfs_session.get_first_result_id().unwrap_or("".to_string());

        let target: Vec<String> = cfs_session
            .get_target_hsm()
            .or_else(|| cfs_session.get_target_xname())
            .unwrap_or_default();

        let cfs_configuration = cfs_session.get_configuration_name().unwrap();

        image_id_cfs_configuration_target_from_cfs_session.push((
            result_id,
            cfs_configuration,
            target,
        ));
    });

    image_id_cfs_configuration_target_from_cfs_session
}

/// Returns a tuple like (image_id, cfs_configuration_name, target) from a list of CFS
/// sessions. Only returns values from CFS sessions with an artifact.result_id value
/// (meaning CFS sessions completed and successful of type image)
pub fn get_image_id_cfs_configuration_target_for_existing_images_tuple_vec(
    cfs_session_vec: Vec<CfsSessionGetResponse>,
) -> Vec<(String, String, Vec<String>)> {
    let mut image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
        Vec::new();

    cfs_session_vec.iter().for_each(|cfs_session| {
        if let Some(result_id) = cfs_session.get_first_result_id() {
            let target: Vec<String> = cfs_session
                .get_target_hsm()
                .or_else(|| cfs_session.get_target_xname())
                .unwrap_or_default();

            let cfs_configuration = cfs_session.get_configuration_name().unwrap();

            image_id_cfs_configuration_target_from_cfs_session.push((
                result_id.to_string(),
                cfs_configuration,
                target,
            ));
        } else {
            image_id_cfs_configuration_target_from_cfs_session.push((
                "".to_string(),
                "".to_string(),
                vec![],
            ));
        }
    });

    image_id_cfs_configuration_target_from_cfs_session
}

/// Return a list of the images ids related with a list of CFS sessions. The result list if
/// filtered to CFS session completed and target def 'image' therefore the length of the
/// resulting list may be smaller than the list of CFS sessions
pub fn get_image_id_from_cfs_session_vec(
    cfs_session_value_vec: &[CfsSessionGetResponse],
) -> Vec<String> {
    cfs_session_value_vec
        .iter()
        .filter(|cfs_session| {
            cfs_session.is_target_def_image()
                && cfs_session.is_success()
                && cfs_session.get_first_result_id().is_some()
        })
        .map(|cfs_session| cfs_session.get_first_result_id().unwrap())
        .collect::<Vec<String>>()
}

/// Wait a CFS session to finish
pub async fn wait_cfs_session_to_finish(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    cfs_session_id: &str,
) {
    let mut i = 0;
    let max = 3000; // Max ammount of attempts to check if CFS session has ended
    loop {
        let cfs_session_vec_rslt = cfs::session::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
            None,
            None,
            Some(&cfs_session_id.to_string()),
            None,
        )
        .await;

        let cfs_session = if let Ok(cfs_session_vec) = cfs_session_vec_rslt {
            cfs_session_vec.first().unwrap().clone()
        } else {
            eprintln!("ERROR - CFS session '{}' missing. Exit", cfs_session_id);
            std::process::exit(1);
        };

        log::debug!("CFS session details:\n{:#?}", cfs_session);

        let cfs_session_status = cfs_session.status.unwrap().session.unwrap().status.unwrap();

        if cfs_session_status != "complete" && i < max {
            print!("\x1B[2K"); // Clear current line
            io::stdout().flush().unwrap();
            println!(
                "Waiting CFS session '{}' with status '{}'. Checking again in 2 secs. Attempt {} of {}.",
                cfs_session_id, cfs_session_status, i, max
            );
            io::stdout().flush().unwrap();

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            i += 1;
        } else {
            println!(
                "CFS session '{}' finished with status '{}'",
                cfs_session_id, cfs_session_status
            );
            break;
        }
    }
}
