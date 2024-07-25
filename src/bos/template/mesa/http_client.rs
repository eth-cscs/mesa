use crate::error::Error;

use super::r#struct::v2::BosSessionTemplate;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&str>,
) -> Result<Vec<BosSessionTemplate>, Error> {
    crate::bos::template::shasta::http_client::v2::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        bos_session_template_id_opt,
    )
    .await
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<BosSessionTemplate>, Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}
