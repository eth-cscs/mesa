use std::error::Error;

use super::r#struct::RecipeGetResponse;

/// Create IMS job ref --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/post_v3_job/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    recipe_id_opt: Option<&str>,
) -> Result<Vec<RecipeGetResponse>, Box<dyn Error>> {
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

    let api_url = if let Some(recipe_id) = recipe_id_opt {
        shasta_base_url.to_owned() + "/ims/v2/recipes" + recipe_id
    } else {
        shasta_base_url.to_owned() + "/ims/v2/recipes"
    };

    let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

    if resp.status().is_success() {
        Ok(resp.json::<Vec<RecipeGetResponse>>().await?)
    } else {
        eprintln!("FAIL request: {:#?}", resp);
        let response: String = resp.text().await?;
        eprintln!("FAIL response: {:#?}", response);
        Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
    }
}
