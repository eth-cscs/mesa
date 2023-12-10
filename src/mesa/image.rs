use crate::shasta::{self, ims::image::Image};

use super::{bos, cfs};

pub async fn filter(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    hsm_group_name_vec: &Vec<String>,
    limit_number: Option<&u8>,
) -> Vec<(Image, String, String)> {
    // println!("DEBUG - HSM grop: {:?}", hsm_group_name_vec);
    let image_vec: Vec<Image> = shasta::ims::image::http_client::get_struct(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        &vec![], // DO NOT FILTER BY HSM GROUP BECAUSE OTHERWISE IT WILL FILTER BY IMAGE NAME WHICH IS
        // WRONG, THERE IS NO CERTAINTY THE HSM GROUP NAME IS GOING TO BE IN THE IMAGE NAME!!!!!
        None,
        None,
        limit_number,
    )
    .await
    .unwrap();

    // We need BOS session templates to find an image created by SAT
    let bos_sessiontemplate_value_vec = shasta::bos::template::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    /* println!(
        "DEBUG - BOS sessiontemplate:\n{:#?}",
        bos_sessiontemplates_value_vec
    ); */

    // We need CFS sessions to find images without a BOS session template
    let cfs_session_value_vec = shasta::cfs::session::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap();

    // println!("DEBUG - CFS session:\n{:#?}", cfs_session_vec);
    let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(
        String,
        String,
        Vec<String>,
    )> = bos::sessiontemplate::utils::get_image_id_cfs_configuration_target_tuple_vec(
        bos_sessiontemplate_value_vec,
    );

    image_id_cfs_configuration_from_bos_sessiontemplate
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    /* println!(
        "DEBUG - bos sessiontemplate paths: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(String, String, Vec<String>)> =
        cfs::session::utils::get_image_id_cfs_configuration_target_tuple_vec(cfs_session_value_vec);

    image_id_cfs_configuration_from_cfs_session_vec
        .retain(|(image_id, _cfs_confguration, _hsm_groups)| !image_id.is_empty());

    /* println!(
        "DEBUG - cfs sessions: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    // let image_id_from_bos_sessiontemplate_vec = bos_sessiontemplates_value_vec

    let mut image_detail_vec: Vec<(Image, String, String)> = Vec::new();

    for image in &image_vec {
        let image_id = image.id.as_ref().unwrap();

        let target_group_name_vec: Vec<String>;
        let cfs_configuration: String;

        if let Some(tuple) = image_id_cfs_configuration_from_bos_sessiontemplate
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.clone().1;
            target_group_name_vec = tuple.2.clone();
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            cfs_configuration = "".to_string();
            target_group_name_vec = vec![];
        } else {
            continue;
        }

        let target_groups = target_group_name_vec.join(", ");

        image_detail_vec.push((
            image.clone(),
            cfs_configuration.to_string(),
            target_groups.clone(),
        ));
    }

    image_detail_vec
}
