use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Link {
    pub path: String,
    pub etag: Option<String>,
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Image {
    pub id: Option<String>,
    pub created: Option<String>,
    pub name: String,
    pub link: Option<Link>,
}

pub mod http_client {

    use std::error::Error;

    use serde_json::Value;

    use crate::mesa;

    use super::Image;

    pub async fn get_all_struct(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Image>, Box<dyn Error>> {
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

        let api_url = shasta_base_url.to_owned() + "/ims/v3/images";

        let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

        if resp.status().is_success() {
            Ok(resp.json::<Vec<Image>>().await?)
        } else {
            return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_all_raw(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Value>, Box<dyn Error>> {
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

        let api_url = shasta_base_url.to_owned() + "/ims/v3/images";

        let resp = client.get(api_url).bearer_auth(shasta_token).send().await?;

        let mut json_response: Value = if resp.status().is_success() {
            serde_json::from_str(&resp.text().await?)?
        } else {
            return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        };

        let image_value_vec: Vec<Value> = json_response
            .as_array_mut()
            .unwrap_or(&mut Vec::new())
            .to_vec();

        Ok(image_value_vec.to_vec())
    }

    pub async fn get_raw(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec: &Vec<String>,
        image_id_opt: Option<&str>,
        image_name_opt: Option<&str>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut image_vec: Vec<Value> =
            get_all_raw(shasta_token, shasta_base_url, shasta_root_cert)
                .await
                .unwrap();

        /* image_value_vec.retain(|image_value| {
            hsm_group_name_vec.iter().any(|hsm_group_name| {
                image_value["name"]
                    .as_str()
                    .unwrap()
                    .to_string()
                    .contains(hsm_group_name)
            })
        }); */

        // Sort images by creation time order ASC
        image_vec.sort_by(|a, b| {
            a["created"]
                .as_str()
                .unwrap()
                .cmp(b["created"].as_str().unwrap())
        });

        // Limiting the number of results to return to client
        if let Some(limit_number) = limit_number_opt {
            image_vec =
                image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
        }

        if let Some(image_id) = image_id_opt {
            image_vec.retain(|image_value| image_value["id"].as_str().unwrap().eq(image_id));
        }

        if let Some(image_name) = image_name_opt {
            image_vec.retain(|image_value| image_value["name"].as_str().unwrap().eq(image_name));
        }

        Ok(image_vec.to_vec())
    }

    /// Fetch IMS image ref --> https://apidocs.svc.cscs.ch/paas/ims/operation/get_v3_image/
    /// If filtering by HSM group, then image name must include HSM group name (It assumms each image
    /// is built for a specific cluster based on ansible vars used by the CFS session). The reason
    /// for this is because CSCS staff deletes all CFS sessions every now and then...
    pub async fn get_struct(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec: &Vec<String>,
        image_id_opt: Option<&str>,
        image_name_opt: Option<&str>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<Image>, Box<dyn Error>> {
        let mut image_vec: Vec<Image> =
            get_all_struct(shasta_token, shasta_base_url, shasta_root_cert)
                .await
                .unwrap();

        /* image_value_vec.retain(|image_value| {
            hsm_group_name_vec.iter().any(|hsm_group_name| {
                image_value["name"]
                    .as_str()
                    .unwrap()
                    .to_string()
                    .contains(hsm_group_name)
            })
        }); */

        // Sort images by creation time order ASC
        image_vec.sort_by(|a, b| a.created.as_ref().unwrap().cmp(b.created.as_ref().unwrap()));

        // Limiting the number of results to return to client
        if let Some(limit_number) = limit_number_opt {
            image_vec =
                image_vec[image_vec.len().saturating_sub(*limit_number as usize)..].to_vec();
        }

        if let Some(image_id) = image_id_opt {
            image_vec.retain(|image_value| image_value.id.as_ref().unwrap().eq(image_id));
        }

        if let Some(image_name) = image_name_opt {
            image_vec.retain(|image_value| image_value.name.eq(image_name));
        }

        Ok(image_vec)
    }

    // Get Image using fuzzy finder, meaning returns any image which name contains a specific
    // string.
    // Used to find an image created through a CFS session and has not been renamed because manta
    // does not rename the images as SAT tool does for the sake of keeping the original image ID in
    // the CFS session which created the image.
    pub async fn get_fuzzy(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec: &Vec<String>,
        image_name_opt: Option<&str>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<(Image, String, String)>, Box<dyn Error>> {
        let mut image_configuration_hsm_group_tuple_vec: Vec<(Image, String, String)> =
            crate::mesa::image::filter(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                hsm_group_name_vec,
                limit_number_opt,
            )
            .await;

        if let Some(image_name) = image_name_opt {
            image_configuration_hsm_group_tuple_vec
                .retain(|(image, _, _)| image.name.contains(image_name));
        }

        Ok(image_configuration_hsm_group_tuple_vec.to_vec())
    }

    // Delete IMS image using CSM API. First does a "soft delete", then a "permanent deletion"
    // soft delete --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_image/
    // permanent deletion --> https://csm12-apidocs.svc.cscs.ch/paas/ims/operation/delete_v3_deleted_image/
    pub async fn delete(
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        image_id: &str,
    ) -> Result<(), Box<dyn Error>> {
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

        // SOFT DELETION
        let api_url = shasta_base_url.to_owned() + "/ims/v3/images/" + image_id;

        let resp = client
            .delete(api_url)
            // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
            .bearer_auth(shasta_token)
            .send()
            .await?;

        if resp.status().is_success() {
            log::debug!("{:#?}", resp);
        } else {
            log::debug!("{:#?}", resp);
            return Err(resp.text().await?.into()); // Black magic conversion from Err(Box::new("my error msg")) which does not
        }

        // PERMANENT DELETION
        let api_url = shasta_base_url.to_owned() + "/ims/v3/deleted/images/" + image_id;

        let resp = client
            .delete(api_url)
            // .get(format!("{}{}", shasta_base_url, "/cfs/v2/configurations"))
            .bearer_auth(shasta_token)
            .send()
            .await?;

        if resp.status().is_success() {
            log::debug!("{:#?}", resp);
            Ok(())
        } else {
            log::debug!("{:#?}", resp);
            Err(resp.text().await?.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }
}
