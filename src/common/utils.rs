use std::time::Instant;

use tokio::task;

use crate::{
    bos::{self, template::http_client::v2::types::BosSessionTemplate},
    cfs::{
        self, component::http_client::v3::types::Component,
        configuration::http_client::v3::types::cfs_configuration_response::CfsConfigurationResponse,
        session::http_client::v3::types::CfsSessionGetResponse,
    },
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
) -> (
    Option<Vec<CfsConfigurationResponse>>,
    Option<Vec<CfsSessionGetResponse>>,
    Option<Vec<BosSessionTemplate>>,
    Option<Vec<Image>>,
) {
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
            cfs::session::get(
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

    let duration = start.elapsed();
    log::info!("Time elapsed to get CFS configurations, CFS sessions, BSS bootparameters and images bundle is: {:?}", duration);

    (
        cfs_configuration_vec,
        cfs_session_vec,
        bos_sessiontemplate_vec,
        ims_image_vec,
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
            cfs::session::get(
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
