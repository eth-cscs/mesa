use std::time::Instant;

use tokio::task;

use crate::{
    bos::{self, template::http_client::v2::types::BosSessionTemplate},
    bss::types::BootParameters,
    cfs::{
        self, component::http_client::v3::types::Component,
        configuration::http_client::v3::types::cfs_configuration_response::CfsConfigurationResponse,
        session::http_client::v3::types::CfsSessionGetResponse,
    },
    error::Error,
    ims::{self, image::http_client::types::Image},
};

pub async fn get_configurations_sessions_bos_sessiontemplates_images(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    get_cfs_configuration: bool,
    get_cfs_session: bool,
    get_bos_sessiontemplate: bool,
    get_ims_image: bool,
) -> Result<
    (
        Option<Vec<CfsConfigurationResponse>>,
        Option<Vec<CfsSessionGetResponse>>,
        Option<Vec<BosSessionTemplate>>,
        Option<Vec<Image>>,
    ),
    Error,
> {
    let start = Instant::now();

    let handle_cfs_configuration_opt = if get_cfs_configuration {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::configuration::http_client::v3::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
            )
            .await
            // .unwrap()
        }))
    } else {
        None
    };

    let handle_cfs_session_opt = if get_cfs_session {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::session::get_and_sort(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            // .unwrap()
        }))
    } else {
        None
    };

    let handle_bos_sessiontemplate_opt = if get_bos_sessiontemplate {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            bos::template::http_client::v2::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            // .unwrap()
        }))
    } else {
        None
    };

    let handle_ims_image_opt = if get_ims_image {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            ims::image::http_client::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            // .unwrap()
        }))
    } else {
        None
    };

    /* let cfs_configuration_vec = if let Some(handle) = handle_cfs_configuration_opt {
        Some(handle.await.unwrap())
    } else {
        None
    }; */
    let cfs_configuration_vec = if let Some(handle) = handle_cfs_configuration_opt {
        match handle.await.unwrap() {
            Ok(cfs_configuration_vec) => Some(cfs_configuration_vec),
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    /* let cfs_session_vec = if let Some(handle) = handle_cfs_session_opt {
        Some(handle.await.unwrap())
    } else {
        None
    }; */
    let cfs_session_vec = if let Some(handle) = handle_cfs_session_opt {
        match handle.await.unwrap() {
            Ok(cfs_session_vec) => Some(cfs_session_vec),
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    /* let bos_sessiontemplate_vec = if let Some(handle) = handle_bos_sessiontemplate_opt {
        Some(handle.await.unwrap())
    } else {
        None
    }; */
    let bos_sessiontemplate_vec = if let Some(handle) = handle_bos_sessiontemplate_opt {
        match handle.await.unwrap() {
            Ok(bos_sessiontemplate_vec) => Some(bos_sessiontemplate_vec),
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    /* let ims_image_vec = if let Some(handle) = handle_ims_image_opt {
        handle.await.unwrap()
    } else {
        None
    }; */
    let ims_image_vec = if let Some(handle) = handle_ims_image_opt {
        match handle.await.unwrap() {
            Ok(ims_image_vec) => Some(ims_image_vec),
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    let duration = start.elapsed();
    log::info!("Time elapsed to get CFS configurations, CFS sessions, BSS bootparameters and images bundle is: {:?}", duration);

    Ok((
        cfs_configuration_vec,
        cfs_session_vec,
        bos_sessiontemplate_vec,
        ims_image_vec,
    ))
}

pub async fn get_configurations_sessions_bos_sessiontemplates_images_components_bootparameters(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    get_cfs_configuration: bool,
    get_cfs_session: bool,
    get_bos_sessiontemplate: bool,
    get_ims_image: bool,
    get_cfs_component: bool,
    get_bss_bootparameters: bool,
) -> (
    Option<Vec<CfsConfigurationResponse>>,
    Option<Vec<CfsSessionGetResponse>>,
    Option<Vec<BosSessionTemplate>>,
    Option<Vec<Image>>,
    Option<Vec<Component>>,
    Option<Vec<BootParameters>>,
) {
    let start = Instant::now();

    let handle_cfs_component_opt = if get_cfs_component {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::component::http_client::v3::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_cfs_configuration_opt = if get_cfs_configuration {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::configuration::http_client::v3::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_cfs_session_opt = if get_cfs_session {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::session::get_and_sort(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_bos_sessiontemplate_opt = if get_bos_sessiontemplate {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            bos::template::http_client::v2::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_ims_image_opt = if get_ims_image {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            ims::image::http_client::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_bss_bootparameters_opt = if get_bss_bootparameters {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            crate::bss::http_client::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                &[],
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let cfs_configuration_vec = if let Some(handle) = handle_cfs_configuration_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let cfs_session_vec = if let Some(handle) = handle_cfs_session_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let bos_sessiontemplate_vec = if let Some(handle) = handle_bos_sessiontemplate_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let ims_image_vec = if let Some(handle) = handle_ims_image_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let cfs_component_vec = if let Some(handle) = handle_cfs_component_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let bss_bootparameters_vec = if let Some(handle) = handle_bss_bootparameters_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let duration = start.elapsed();
    log::info!("Time elapsed to get CFS configurations, CFS sessions, BOS sessiontemplate, IMS images and BSS bootparameters bundle is: {:?}", duration);

    (
        cfs_configuration_vec,
        cfs_session_vec,
        bos_sessiontemplate_vec,
        ims_image_vec,
        cfs_component_vec,
        bss_bootparameters_vec,
    )
}

pub async fn get_configurations_sessions_bos_sessiontemplates_images_components(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    get_cfs_configuration: bool,
    get_cfs_session: bool,
    get_bos_sessiontemplate: bool,
    get_ims_image: bool,
    get_cfs_component: bool,
) -> (
    Option<Vec<CfsConfigurationResponse>>,
    Option<Vec<CfsSessionGetResponse>>,
    Option<Vec<BosSessionTemplate>>,
    Option<Vec<Image>>,
    Option<Vec<Component>>,
) {
    let start = Instant::now();

    let handle_cfs_component_opt = if get_cfs_component {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::component::http_client::v3::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_cfs_configuration_opt = if get_cfs_configuration {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::configuration::http_client::v3::get(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_cfs_session_opt = if get_cfs_session {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::session::get_and_sort(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_bos_sessiontemplate_opt = if get_bos_sessiontemplate {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            bos::template::http_client::v2::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let handle_ims_image_opt = if get_ims_image {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            ims::image::http_client::get_all(
                &shasta_token_string,
                &shasta_base_url_string,
                &shasta_root_cert_vec,
            )
            .await
            .unwrap()
        }))
    } else {
        None
    };

    let cfs_configuration_vec = if let Some(handle) = handle_cfs_configuration_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let cfs_session_vec = if let Some(handle) = handle_cfs_session_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let bos_sessiontemplate_vec = if let Some(handle) = handle_bos_sessiontemplate_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let ims_image_vec = if let Some(handle) = handle_ims_image_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let cfs_component_vec = if let Some(handle) = handle_cfs_component_opt {
        Some(handle.await.unwrap())
    } else {
        None
    };

    let duration = start.elapsed();
    log::info!("Time elapsed to get CFS configurations, CFS sessions, BSS bootparameters and images bundle is: {:?}", duration);

    (
        cfs_configuration_vec,
        cfs_session_vec,
        bos_sessiontemplate_vec,
        ims_image_vec,
        cfs_component_vec,
    )
}
