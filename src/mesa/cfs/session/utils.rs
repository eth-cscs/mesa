use serde_json::Value;

pub fn get_image_id_cfs_configuration_target_tuple_vec(
    cfs_session_value_vec: Vec<Value>,
) -> Vec<(String, String, Vec<String>)> {
    let mut image_id_cfs_configuration_target_from_cfs_session: Vec<(String, String, Vec<String>)> =
        Vec::new();

    cfs_session_value_vec
        .iter()
        .for_each(|cfs_session| {
            if let Some(result_id) = cfs_session.pointer("/status/artifacts/0/result_id") {
                let target: Vec<String> =
                    if let Some(target_groups) = cfs_session.pointer("/target/groups") {
                        target_groups
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|group| group["name"].as_str().unwrap().to_string())
                            .collect()
                    } else if let Some(ansible_limit) = cfs_session.pointer("/ansible/limit") {
                        ansible_limit
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|xname| xname.as_str().unwrap().to_string())
                            .collect()
                    } else {
                        vec![]
                    };

                image_id_cfs_configuration_target_from_cfs_session.push((
                    result_id.as_str().unwrap().to_string(),
                    cfs_session
                        .pointer("/configuration/name")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    target,
                ));
            } else {
                image_id_cfs_configuration_target_from_cfs_session.push(("".to_string(), "".to_string(), vec![]));
            }
        });

    image_id_cfs_configuration_target_from_cfs_session
}
