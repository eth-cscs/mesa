use crate::shasta::{self, ims::image::Image};

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
    let bos_sessiontemplates_value_vec = shasta::bos::template::http_client::filter(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        hsm_group_name_vec,
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
    let cfs_session_vec = shasta::cfs::session::http_client::filter(
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

    let mut image_id_cfs_configuration_from_bos_sessiontemplate: Vec<(&str, &str, Vec<&str>)> =
        bos_sessiontemplates_value_vec
            .iter()
            .map(|bos_sessiontemplate| {
                if let Some(path) = bos_sessiontemplate.pointer("/boot_sets/compute/path") {
                    let target: Vec<&str> = if let Some(node_groups) =
                        bos_sessiontemplate.pointer("/boot_sets/compute/node_groups")
                    {
                        node_groups
                            .as_array()
                            .unwrap()
                            .into_iter()
                            .map(|target_group| target_group.as_str().unwrap())
                            .collect()
                    } else if let Some(node_list) =
                        bos_sessiontemplate.pointer("/boot_sets/compute/node_list")
                    {
                        node_list
                            .as_array()
                            .unwrap()
                            .into_iter()
                            .map(|target_group| target_group.as_str().unwrap())
                            .collect()
                    } else {
                        vec![]
                    };

                    (
                        path.as_str()
                            .unwrap()
                            .strip_prefix("s3://boot-images/")
                            .unwrap()
                            .strip_suffix("/manifest.json")
                            .unwrap(),
                        bos_sessiontemplate
                            .pointer("/cfs/configuration")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                        target,
                    )
                } else if let Some(path) = bos_sessiontemplate.pointer("/boot_sets/uan/path") {
                    (
                        path.as_str()
                            .unwrap()
                            .strip_prefix("s3://boot-images/")
                            .unwrap()
                            .strip_suffix("/manifest.json")
                            .unwrap(),
                        bos_sessiontemplate
                            .pointer("/cfs/configuration")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                        vec![],
                    )
                } else {
                    ("", "", vec![])
                }
            })
            .collect();

    image_id_cfs_configuration_from_bos_sessiontemplate
        .retain(|(image_id, _cfs_configuration, _hsm_groups)| !image_id.is_empty());

    /* println!(
        "DEBUG - bos sessiontemplate paths: {:#?}",
        image_id_cfs_configuration_from_bos_sessiontemplate
    ); */

    let mut image_id_cfs_configuration_from_cfs_session_vec: Vec<(&str, &str, Vec<&str>)> =
        cfs_session_vec
            .iter()
            .map(|cfs_session| {
                if let Some(result_id) = cfs_session.pointer("/status/artifacts/0/result_id") {
                    let target: Vec<&str> =
                        if let Some(target_groups) = cfs_session.pointer("/target/groups") {
                            target_groups
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|group| group["name"].as_str().unwrap())
                                .collect()
                        } else if let Some(ansible_limit) = cfs_session.pointer("/ansible/limit") {
                            ansible_limit
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|xname| xname.as_str().unwrap())
                                .collect()
                        } else {
                            vec![]
                        };

                    (
                        result_id.as_str().unwrap(),
                        cfs_session
                            .pointer("/configuration/name")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                        target,
                    )
                } else {
                    ("", "", vec![])
                }
            })
            .collect();

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

        let target_group_name_vec: Vec<&str>;
        let cfs_configuration: &str;

        if let Some(tuple) = image_id_cfs_configuration_from_bos_sessiontemplate
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.1;
            target_group_name_vec = tuple.2.clone();
        } else if let Some(tuple) = image_id_cfs_configuration_from_cfs_session_vec
            .iter()
            .find(|tuple| tuple.0.eq(image_id))
        {
            cfs_configuration = tuple.1;
            target_group_name_vec = tuple.2.clone();
        } else if hsm_group_name_vec
            .iter()
            .any(|hsm_group_name| image.name.contains(hsm_group_name))
        {
            cfs_configuration = "";
            target_group_name_vec = vec![];
        } else {
            continue;
        }

        let target_groups = target_group_name_vec.join(", ");

        image_detail_vec.push((image.clone(), cfs_configuration.to_string(), target_groups.clone()));
    }

    image_detail_vec
}
