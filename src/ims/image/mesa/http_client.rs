use crate::ims::image::r#struct::Image;

pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    image_id_opt: Option<&str>,
) -> Result<Vec<Image>, reqwest::Error> {
    let response_rslt = crate::ims::image::shasta::http_client::get_raw(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        image_id_opt,
    )
    .await;

    let image_vec: Vec<Image> = match response_rslt {
        Ok(response) => {
            if image_id_opt.is_none() {
                response.json::<Vec<Image>>().await.unwrap()
            } else {
                vec![response.json::<Image>().await.unwrap()]
            }
        }
        Err(error) => return Err(error),
    };

    Ok(image_vec)
}

pub async fn get_all(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
) -> Result<Vec<Image>, reqwest::Error> {
    get(shasta_token, shasta_base_url, shasta_root_cert, None).await
}
