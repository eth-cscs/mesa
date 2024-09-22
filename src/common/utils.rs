use tokio::task;

use crate::{
    bos::{self, template::mesa::r#struct::v2::BosSessionTemplate},
    cfs::{
        self,
        configuration::mesa::r#struct::cfs_configuration_response::v2::CfsConfigurationResponse,
        session::mesa::r#struct::v3::CfsSessionGetResponse,
    },
    ims::{self, image::r#struct::Image},
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
    let handle_cfs_configuration_opt = if get_cfs_configuration {
        let shasta_token_string = shasta_token.to_string();
        let shasta_base_url_string = shasta_base_url.to_string();
        let shasta_root_cert_vec = shasta_root_cert.to_vec();

        Some(task::spawn(async move {
            cfs::configuration::mesa::http_client::get(
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
            cfs::session::mesa::http_client::get(
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
            bos::template::mesa::http_client::get_all(
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
            ims::image::mesa::http_client::get_all(
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

    (
        cfs_configuration_vec,
        cfs_session_vec,
        bos_sessiontemplate_vec,
        ims_image_vec,
    )
}
