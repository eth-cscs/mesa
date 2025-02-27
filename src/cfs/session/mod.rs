pub mod http_client;
pub mod utils;

use crate::cfs;
use http_client::v3::types::{CfsSessionGetResponse, CfsSessionPostRequest};

use crate::{
    common::{
        kubernetes::{self, print_cfs_session_logs},
        vault::http_client::fetch_shasta_k8s_secrets,
    },
    error::Error,
};

/// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
/// Returns list of CFS sessions ordered by start time.
/// This methods filter by either HSM group name or HSM group members or both
pub async fn get_and_sort(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    min_age_opt: Option<&String>,
    max_age_opt: Option<&String>,
    status_opt: Option<&String>,
    session_name_opt: Option<&String>,
    is_succeded_opt: Option<bool>,
) -> Result<Vec<CfsSessionGetResponse>, Error> {
    let mut cfs_session_vec = cfs::session::http_client::v3::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        session_name_opt,
        None,
        None,
        min_age_opt.cloned(),
        max_age_opt.cloned(),
        status_opt.cloned(),
        None,
        is_succeded_opt,
        None,
    )
    .await?;

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

    Ok(cfs_session_vec)
}

pub async fn post(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    session: &CfsSessionPostRequest,
) -> Result<CfsSessionGetResponse, Error> {
    log::info!("Create CFS session '{}'", session.name);
    log::debug!("Create CFS session request payload:\n{:#?}", session);

    cfs::session::http_client::v3::post(shasta_token, shasta_base_url, shasta_root_cert, session)
        .await
}

pub async fn post_sync(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    vault_base_url: &str,
    site_name: &str,
    k8s_api_url: &str,
    session: &CfsSessionPostRequest,
    watch_logs: bool,
) -> Result<CfsSessionGetResponse, Error> {
    let cfs_session: CfsSessionGetResponse =
        cfs::session::post(shasta_token, shasta_base_url, shasta_root_cert, session).await?;

    let cfs_session_name: String = cfs_session.name.unwrap();

    // FIXME: refactor becase this code is duplicated in command `manta apply sat-file` and also in
    // `manta logs`
    if watch_logs {
        log::info!("Fetching logs ...");
        let shasta_k8s_secrets =
            fetch_shasta_k8s_secrets(shasta_token, vault_base_url, site_name).await?;

        let client = kubernetes::get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
            .await
            .unwrap();

        let _ = print_cfs_session_logs(client, &cfs_session_name).await;
    }

    // User does not want the CFS logs but we still need to wayt the CFS session to
    // finis. Wait till the CFS session finishes
    utils::wait_cfs_session_to_finish(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &cfs_session_name,
    )
    .await?;

    // Get most recent CFS session status
    let cfs_session: CfsSessionGetResponse = get_and_sort(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        None,
        None,
        None,
        Some(&cfs_session_name),
        None,
    )
    .await?
    .first()
    .unwrap()
    .clone();

    Ok(cfs_session)
}
