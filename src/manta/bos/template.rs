// use serde_json::Value;

/* pub fn get_image_id_from_bos_sessiontemplate_vec(
    bos_sessiontemplate_value_vec: &[Value],
) -> Vec<String> {
    bos_sessiontemplate_value_vec
        .into_iter()
        .map(|bos_sessiontemplate_value| {
            bos_sessiontemplate_value["boot_sets"]
                .as_object()
                .unwrap()
                .into_iter()
                .map(|(_, boot_set_param_value)| {
                    boot_set_param_value["path"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("s3://boot-images/")
                        .unwrap()
                        .strip_suffix("/manifest.json")
                        .unwrap()
                        .to_string()
                })
        })
        .flatten()
        .collect()
} */
