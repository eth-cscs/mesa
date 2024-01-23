use std::error::Error;

use serde_json::Value;

use crate::ims::image::r#struct::{Image, ImsImageRecord2Update};

pub async fn filter(image_vec: &mut [Image]) {
    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| a.created.as_ref().unwrap().cmp(b.created.as_ref().unwrap()));
}

/// update an IMS image record --> https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/ims.md#post_v2_image
pub async fn update_image(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    ims_image_id: &String,
    ims_link: &ImsImageRecord2Update,
) -> Result<Value, Box<dyn Error>> {
    let client;

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    if std::env::var("SOCKS5").is_ok() {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

        // rest client to authenticate
        client = client_builder.proxy(socks5proxy).build()?;
    } else {
        client = client_builder.build()?;
    }

    let api_url = shasta_base_url.to_owned() + "/ims/v3/images/" + &ims_image_id;

    let resp = client
        .patch(api_url)
        .header("Authorization", format!("Bearer {}", shasta_token))
        .json(&ims_link)
        .send()
        .await?;

    let json_response: Value;

    if resp.status().is_success() {
        log::debug!("{:#?}", resp);
        json_response = serde_json::from_str(&resp.text().await?)?;
        Ok(json_response)
    } else {
        log::debug!("{:#?}", resp);
        Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}
