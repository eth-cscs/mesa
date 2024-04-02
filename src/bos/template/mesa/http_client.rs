use crate::error::Error;

use super::r#struct::v1::BosSessionTemplate;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    bos_session_template_id_opt: Option<&String>,
) -> Result<Vec<BosSessionTemplate>, Error> {
    crate::bos::template::shasta::http_client::v1::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        bos_session_template_id_opt,
    )
    .await
    .map_err(|error| Error::NetError(error))
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<BosSessionTemplate>, Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}
