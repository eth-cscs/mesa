use serde_json::Value;

pub fn get_image_id_from_bos_sessiontemplate_related_to_cfs_configuration(
    bos_sessiontemplate_value_vec: &Vec<Value>,
    // cfs_configuration: &str,
) -> Vec<String> {
    let image_id_compute_iter = bos_sessiontemplate_value_vec
        .iter()
        .filter(|bos_sessiontemplate| {
            /* bos_sessiontemplate
            .pointer("/cfs/configuration")
            .unwrap()
            .eq(cfs_configuration)
            && */
            bos_sessiontemplate
                .pointer("/boot_sets/compute/path")
                .is_some()
        })
        .map(|bos_sessiontemplate| {
            bos_sessiontemplate
                .pointer("/boot_sets/compute/path")
                .unwrap()
                .as_str()
                .unwrap()
                .strip_prefix("s3://boot-images/")
                .unwrap()
                .strip_suffix("/manifest.json")
                .unwrap()
                .to_string()
        });

    let image_id_uan_iter = bos_sessiontemplate_value_vec
        .iter()
        .filter(|bos_sessiontemplate| {
            /* bos_sessiontemplate
            .pointer("/cfs/configuration")
            .unwrap()
            .eq(cfs_configuration)
            && */
            bos_sessiontemplate.pointer("/boot_sets/uan/path").is_some()
        })
        .map(|bos_sessiontemplate| {
            bos_sessiontemplate
                .pointer("/boot_sets/uan/path")
                .unwrap()
                .as_str()
                .unwrap()
                .strip_prefix("s3://boot-images/")
                .unwrap()
                .strip_suffix("/manifest.json")
                .unwrap()
                .to_string()
        });

    let image_id_from_bos_session_template = image_id_compute_iter
        .chain(image_id_uan_iter)
        .collect::<Vec<String>>();

    image_id_from_bos_session_template
}
