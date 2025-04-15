use crate::{
    bss::types::BootParameters,
    cfs::{
        self, component::http_client::v3::types::Component,
        session::utils::get_list_xnames_related_to_session,
    },
    error::Error,
    ims,
};
use dialoguer::{theme::ColorfulTheme, Confirm};

pub async fn exec(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_available_vec: Vec<String>,
    cfs_session_name: &str,
    dry_run: bool,
    assume_yes: bool,
) -> Result<(), Error> {
    log::info!("Deleting session '{}'", cfs_session_name);

    // Get collectives (CFS configuration, CFS session, BOS session template, IMS image and CFS component)
    let get_cfs_configuration = false;
    let get_cfs_session = true;
    let get_bos_sessiontemplate = false;
    let get_ims_image = false;
    let get_cfs_component = true;
    let get_bss_bootparameters = true;

    let (
        _cfs_configuration_vec_opt,
        cfs_session_vec_opt,
        _bos_sessiontemplate_vec_opt,
        _image_vec_opt,
        cfs_component_vec_opt,
        bss_bootparameters_vec_opt
    ) = crate::common::utils::get_configurations_sessions_bos_sessiontemplates_images_components_bootparameters(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        get_cfs_configuration,
        get_cfs_session,
        get_bos_sessiontemplate,
        get_ims_image,
        get_cfs_component,
        get_bss_bootparameters,
    )
    .await;

    // Validate:
    // - Check CFS session belongs to a cluster available to the user
    // - Check CFS session to delete exists
    // - CFS configuration related to CFS session is not being used to create an image
    // - CFS configuration related to CFS session is not a desired configuration
    //
    // Get CFS session to delete
    // Filter CFS sessions based on use input
    let mut cfs_session_vec = cfs_session_vec_opt.unwrap_or_default();

    // Check CFS session belongs to a cluster the user has access to (filter sessions by HSM
    // group)
    cfs::session::utils::filter_by_hsm(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &mut cfs_session_vec,
        &hsm_group_available_vec,
        None,
        false,
    )
    .await?;

    // Check CFS session to delete exists (filter sessions by name)
    let cfs_session = cfs_session_vec
        .iter()
        .find(|cfs_session| cfs_session.name.eq(&Some(cfs_session_name.to_string())))
        .ok_or_else(|| {
            Error::Message(format!(
                "CFS session '{}' not found. Exit",
                cfs_session_name
            ))
        })?;

    // Get xnames related to CFS session to delete:
    // - xnames belonging to HSM group related to CFS session
    // - xnames in CFS session
    let xname_vec = get_list_xnames_related_to_session(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        cfs_session.clone(),
    )
    .await?;

    let cfs_session_target_definition = cfs_session.get_target_def().unwrap();

    // DELETE DATA
    //
    // * if session is of type dynamic (runtime session) then:
    // Get retry_policy
    if cfs_session_target_definition == "dynamic" {
        // The CFS session is of type 'target dynamic' (runtime CFS batcher) - cancel session by
        // setting error_count to retry_policy value
        log::info!("CFS session target definition is 'dynamic'.");

        let cfs_global_options = cfs::component::http_client::v3::get_options(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
        )
        .await?;

        let retry_policy = cfs_global_options["default_batcher_retry_policy"]
            .as_u64()
            .unwrap();

        if !assume_yes {
            // Ask user for confirmation
            let user_msg = format!(
                "Session '{}' will get canceled:\nDo you want to continue?",
                cfs_session_name,
            );
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(user_msg)
                .interact()
                .unwrap()
            {
                log::info!("Continue",);
            } else {
                println!("Cancelled by user. Aborting.");
                return Ok(());
            }
        }
        let _ = cancel_session(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            xname_vec,
            cfs_component_vec_opt,
            retry_policy,
            dry_run,
        )
        .await?;
    } else if cfs_session_target_definition == "image" {
        // The CFS session is not of type 'target dynamic' (runtime CFS batcher)
        let image_created_by_cfs_session_vec = cfs_session.get_result_id_vec();
        if !image_created_by_cfs_session_vec.is_empty() {
            if !assume_yes {
                // Ask user for confirmation
                let user_msg = format!(
                    "Images listed below which will get deleted:\n{}\nDo you want to continue?",
                    image_created_by_cfs_session_vec.join("\n"),
                );
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(user_msg)
                    .interact()
                    .unwrap()
                {
                    log::info!("Continue",);
                } else {
                    println!("Cancelled by user. Aborting.");
                    return Ok(());
                }
            }

            let bss_bootparameters_vec = bss_bootparameters_vec_opt.unwrap_or_default();

            delete_images(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &image_created_by_cfs_session_vec,
                &bss_bootparameters_vec,
                dry_run,
            )
            .await?;
        }
    } else {
        return Err(Error::Message(format!(
            "CFS session target definition is '{}'. Don't know how to continue. Exit",
            cfs_session_target_definition
        )));
    };

    // Delete CFS session
    log::info!("Delete CFS session '{}'", cfs_session_name);
    if !dry_run {
        let _ = cfs::session::http_client::v3::delete(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &cfs_session_name,
        )
        .await;
    } else {
        println!("Delete CFS session '{}'", cfs_session_name);
    }

    println!("Session '{cfs_session_name}' has been deleted.");

    Ok(())
}

async fn delete_images(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_created_by_cfs_session_vec: &[String],
    bss_bootparameters_vec_opt: &[BootParameters],
    dry_run: bool,
) -> Result<(), Error> {
    // Delete images
    for image_id in image_created_by_cfs_session_vec {
        let is_image_boot_node = bss_bootparameters_vec_opt
            .iter()
            .any(|boot_parameters| boot_parameters.get_boot_image().eq(image_id));

        if !is_image_boot_node {
            if !dry_run {
                ims::image::http_client::delete(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    image_id,
                )
                .await?;
            } else {
                println!(
                    "DRYRUN - CFS session target definition is 'image'. Deleting image '{}'",
                    image_id
                );
            }
        } else {
            println!(
                "Image '{}' is a boot node image. It will not be deleted.",
                image_id
            );
        }
    }

    Ok(())
}

async fn cancel_session(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname_vec: Vec<String>,
    cfs_component_vec_opt: Option<Vec<Component>>,
    retry_policy: u64,
    dry_run: bool,
) -> Result<(), Error> {
    // Set CFS components error_count == retry_policy so CFS batcher stops retrying running
    log::info!(
        "Set 'error_count' {} to xnames {:?}",
        retry_policy,
        xname_vec
    );

    // Update CFS component error_count
    let cfs_component_vec: Vec<Component> = cfs_component_vec_opt
        .expect("No CFS components")
        .iter()
        .filter(|cfs_component| {
            xname_vec.contains(
                &cfs_component
                    .id
                    .as_ref()
                    .expect("CFS component found but it has no id???"),
            )
        })
        .cloned()
        .collect();

    // Convert CFS components to another struct we can use for CFS component PUT API
    let cfs_component_request_vec = cfs_component_vec;

    log::info!(
        "Update error count on nodes {:?} to {}",
        xname_vec,
        retry_policy
    );

    if !dry_run {
        let _ = cfs::component::http_client::v3::put_component_list(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            cfs_component_request_vec,
        )
        .await?;
    } else {
        println!("Update error count on nodes {:?}", xname_vec);
    }

    Ok(())
}
