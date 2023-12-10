use serde_json::Value;

/* pub fn get_image_id_from_bos_sessiontemplate_vec(
    bos_sessiontemplate_value_vec: &[Value],
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

    image_id_compute_iter
        .chain(image_id_uan_iter)
        .collect::<Vec<String>>()
} */

pub fn get_image_id_from_bos_sessiontemplate_vec(
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

    /* let aux3: Vec<&str> = aux
    .iter()
    .copied()
    .map(|boot_set_param_value| {
        boot_set_param_value
            .iter()
            .map(|(_, boot_set_param_value)| {
                boot_set_param_value["path"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("s3://boot-images/")
                    .unwrap()
                    .strip_suffix("/manifest.json")
                    .unwrap()
            })
    })
    .flatten()
    .collect(); */

    /* let image_id_compute_iter = bos_sessiontemplate_value_vec
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

    let aux2 = image_id_compute_iter
        .chain(image_id_uan_iter)
        .collect::<Vec<String>>(); */
}
