pub fn exec() -> Configuration {
    let shasta_token = backend.get_api_token(&site_name).await?;

    // FIXME: gitea auth token should be calculated before calling this function
    let gitea_token = crate::common::vault::http_client::fetch_shasta_vcs_token(
        &shasta_token,
        vault_base_url.expect("ERROR - vault base url is mandatory"),
        &site_name,
    )
    .await
    .unwrap();

    let hsm_group_name_arg_rslt = cli_get_configuration.try_get_one("hsm-group");

    let target_hsm_group_vec = get_groups_available(
        &backend,
        &shasta_token,
        hsm_group_name_arg_rslt.unwrap_or(None),
        settings_hsm_group_name_opt,
    )
    .await?;

    let limit: Option<&u8> = if let Some(true) = cli_get_configuration.get_one("most-recent") {
        Some(&1)
    } else {
        cli_get_configuration.get_one::<u8>("limit")
    };

    get_configuration::exec(
        &backend,
        gitea_base_url,
        &gitea_token,
        &shasta_token,
        shasta_base_url,
        shasta_root_cert,
        cli_get_configuration.get_one::<String>("name"),
        cli_get_configuration.get_one::<String>("pattern"),
        &target_hsm_group_vec,
        limit,
        cli_get_configuration.get_one("output"),
        &site_name,
    )
    .await;
}
