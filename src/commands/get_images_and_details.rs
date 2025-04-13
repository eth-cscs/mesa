use crate::{
    error::Error,
    ims::image::{self, http_client::types::Image},
};

pub async fn get_images_and_details(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &[String],
    id_opt: Option<&String>,
    limit_number: Option<&u8>,
) -> Result<Vec<(Image, String, String, bool)>, Error> {
    let mut image_vec: Vec<Image> = image::http_client::get(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        id_opt.map(|elem| elem.as_str()),
    )
    .await
    .unwrap();

    let image_detail_vec_rslt: Result<Vec<(Image, String, String, bool)>, Error> =
        image::utils::get_image_cfs_config_name_hsm_group_name(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &mut image_vec,
            hsm_group_name_vec,
            limit_number,
        )
        .await
        .map_err(|e| Error::Message(format!("ERROR - Failed to get image details: {}", e)));

    image_detail_vec_rslt
}
