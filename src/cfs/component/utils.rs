use crate::cfs::component::http_client::v3::r#struct::Component;

pub async fn update_component_desired_configuration(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xname: &str,
    desired_configuration: &str,
    enabled: bool,
) {
    let component = Component {
        id: Some(xname.to_string()),
        desired_config: Some(desired_configuration.to_string()),
        state: None,
        state_append: None,
        error_count: None,
        retry_policy: None,
        enabled: Some(enabled),
        tags: None,
        configuration_status: None,
        logs: None,
    };

    let _ = crate::cfs::component::http_client::v3::patch_component(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        component,
    )
    .await;
}

pub async fn update_component_list_desired_configuration(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    xnames: Vec<String>,
    desired_configuration: &str,
    enabled: bool,
) {
    let mut component_list = Vec::new();

    for xname in xnames {
        let component = Component {
            id: Some(xname.to_string()),
            desired_config: Some(desired_configuration.to_string()),
            state: None,
            state_append: None,
            error_count: None,
            retry_policy: None,
            enabled: Some(enabled),
            tags: None,
            configuration_status: None,
            logs: None,
        };

        component_list.push(component);
    }

    let _ = crate::cfs::component::http_client::v3::patch_component_list(
        shasta_token,
        shasta_base_url,
        shasta_root_cert,
        component_list,
    )
    .await;
}
