use std::error::Error;

use crate::ims::image::r#struct::Image;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
) -> Result<Vec<Image>, Box<dyn Error>> {
    let resp = crate::ims::image::shasta::http_client::get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        image_id_opt,
    )
    .await
    .unwrap();

    let mut image_vec: Vec<Image> = if resp.status().is_success() {
        resp.json::<Vec<Image>>().await?
    } else {
        let response = resp.text().await;
        log::error!("{:#?}", response);
        return Err(response?.into());
    };

    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| {
        a.created
            .as_ref()
            .unwrap()
            .cmp(&b.created.as_ref().unwrap())
    });

    Ok(image_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Image>, Box<dyn Error>> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}
