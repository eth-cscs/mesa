use std::collections::HashMap;

use backend_dispatcher::{
    contracts::BackendTrait,
    error::Error,
    types::{BootParameters as FrontEndBootParameters, Group as FrontEndGroup},
};
use serde_json::Value;

use crate::{
    bss,
    common::authentication,
    hsm::{self, group::types::Member},
    pcs,
};

pub struct Csm {
    base_url: String,
    root_cert: Vec<u8>,
}

impl Csm {
    pub fn new(base_url: &str, root_cert: &[u8]) -> Self {
        Self {
            base_url: base_url.to_string(),
            root_cert: root_cert.to_vec(),
        }
    }
}

impl BackendTrait for Csm {
    fn test_backend_trait(&self) -> String {
        println!("in mesa backend");
        "in mesa backend".to_string()
    }

    async fn get_api_token(&self, site_name: &str) -> Result<String, Error> {
        // FIXME: this is not nice but authentication/authorization will potentially move out to an
        // external crate since this is type of logic is external to each site ...
        let base_url = self
            .base_url
            .strip_suffix("/apis")
            .unwrap_or(&self.base_url);
        let keycloak_base_url = base_url.to_string() + "/keycloak";

        authentication::get_api_token(
            &self.base_url,
            &self.root_cert,
            &keycloak_base_url,
            site_name,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_group_name_available(&self, auth_token: &str) -> Result<Vec<String>, Error> {
        // Get HSM groups/Keycloak roles the user has access to from JWT token
        let mut realm_access_role_vec = crate::common::jwt_ops::get_roles(auth_token);

        // remove keycloak roles not related with HSM groups
        realm_access_role_vec
            .retain(|role| !role.eq("offline_access") && !role.eq("uma_authorization"));

        // Remove site wide HSM groups like 'alps', 'prealps', 'alpsm', etc because they pollute
        // the roles to check if a user has access to individual compute nodes
        //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
        //wide operations instead of using roles
        let mut realm_access_role_filtered_vec =
            hsm::group::hacks::filter_system_hsm_group_names(realm_access_role_vec.clone());
        realm_access_role_filtered_vec.sort();

        if !realm_access_role_vec.is_empty() {
            Ok(realm_access_role_vec)
        } else {
            let all_hsm_groups_rslt = self.get_all_groups(auth_token).await;

            let mut all_hsm_groups = all_hsm_groups_rslt?
                .iter()
                .map(|hsm_value| hsm_value.label.clone())
                .collect::<Vec<String>>();

            all_hsm_groups.sort();

            Ok(all_hsm_groups)
        }
    }

    // FIXME: rename function to 'get_hsm_group_members'
    async fn get_member_vec_from_group_name_vec(
        &self,
        auth_token: &str,
        hsm_group_name_vec: Vec<String>,
    ) -> Result<Vec<String>, Error> {
        // FIXME: try to merge functions get_member_vec_from_hsm_name_vec_2 and get_member_vec_from_hsm_name_vec
        hsm::group::utils::get_member_vec_from_hsm_name_vec(
            auth_token,
            &self.base_url,
            &self.root_cert,
            hsm_group_name_vec,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn post_members(
        &self,
        auth_token: &str,
        group_label: &str,
        xnames: &[&str],
    ) -> Result<(), Error> {
        let member = Member {
            ids: Some(xnames.into_iter().map(|xname| xname.to_string()).collect()),
        };

        hsm::group::http_client::post_members(
            auth_token,
            &self.base_url,
            &self.root_cert,
            group_label,
            member,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn add_members_to_group(
        &self,
        auth_token: &str,
        group_label: &str,
        members: Vec<&str>,
    ) -> Result<Vec<String>, Error> {
        hsm::group::utils::add_members(
            auth_token,
            &self.base_url,
            &self.root_cert,
            group_label,
            members.to_vec(),
            false,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn delete_member_from_group(
        &self,
        auth_token: &str,
        group_label: &str,
        xname: &str,
    ) -> Result<(), Error> {
        hsm::group::http_client::delete_member(
            auth_token,
            &self.base_url,
            &self.root_cert,
            group_label,
            xname,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn update_group_members(
        &self,
        auth_token: &str,
        group_name: &str,
        members_to_remove: &Vec<String>,
        members_to_add: &Vec<String>,
    ) -> Result<(), Error> {
        hsm::group::utils::update_hsm_group_members(
            auth_token,
            &self.base_url,
            &self.root_cert,
            group_name,
            members_to_remove,
            members_to_add,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_all_groups(&self, auth_token: &str) -> Result<Vec<FrontEndGroup>, Error> {
        // Get all HSM groups
        let hsm_group_backend_vec =
            hsm::group::http_client::get_all(auth_token, &self.base_url, &self.root_cert)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;

        // Convert all HSM groups from mesa to infra
        let hsm_group_vec = hsm_group_backend_vec
            .into_iter()
            .map(hsm::group::types::Group::into)
            .collect();

        Ok(hsm_group_vec)
    }

    async fn get_group(&self, auth_token: &str, hsm_name: &str) -> Result<FrontEndGroup, Error> {
        // Get all HSM groups
        let hsm_group_backend_vec = hsm::group::http_client::get(
            auth_token,
            &self.base_url,
            &self.root_cert,
            Some(&hsm_name.to_string()),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        // Error if more than one HSM group found
        if hsm_group_backend_vec.len() > 1 {
            return Err(Error::Message(format!(
                "ERROR - multiple HSM groups with name '{}' found. Exit",
                hsm_name
            )));
        }

        let hsm_group_backend = hsm_group_backend_vec.first().unwrap().to_owned();

        let hsm_group: FrontEndGroup = hsm_group_backend.into();

        Ok(hsm_group)
    }

    async fn add_group(
        &self,
        auth_token: &str,
        group: FrontEndGroup,
    ) -> Result<FrontEndGroup, Error> {
        let group_csm = hsm::group::http_client::post(
            &auth_token,
            &self.base_url,
            &self.root_cert,
            group.into(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        let group: FrontEndGroup = group_csm.into();

        Ok(group)
    }

    async fn delete_group(&self, auth_token: &str, label: &str) -> Result<Value, Error> {
        hsm::group::http_client::delete_hsm_group(
            auth_token,
            &self.base_url,
            &self.root_cert,
            &label.to_string(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    // HSM/GROUP
    async fn migrate_group_members(
        &self,
        shasta_token: &str,
        target_hsm_group_name: &str,
        parent_hsm_group_name: &str,
        new_target_hsm_members: Vec<&str>,
    ) -> Result<(Vec<String>, Vec<String>), Error> {
        hsm::group::utils::migrate_hsm_members(
            shasta_token,
            &self.base_url,
            &self.root_cert,
            target_hsm_group_name,
            parent_hsm_group_name,
            new_target_hsm_members,
            true,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn power_on_sync(&self, auth_token: &str, nodes: &[String]) -> Result<Value, Error> {
        let operation = "on";

        pcs::transitions::http_client::post_block(
            &self.base_url,
            auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn power_off_sync(
        &self,
        auth_token: &str,
        nodes: &[String],
        force: bool,
    ) -> Result<serde_json::Value, Error> {
        let operation = if force { "force-off" } else { "soft-off" };

        pcs::transitions::http_client::post_block(
            &self.base_url,
            auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn power_reset_sync(
        &self,
        auth_token: &str,
        nodes: &[String],
        force: bool,
    ) -> Result<serde_json::Value, Error> {
        let operation = if force {
            "hard-restart"
        } else {
            "soft-restart"
        };

        pcs::transitions::http_client::post_block(
            &self.base_url,
            auth_token,
            &self.root_cert,
            operation,
            &nodes.to_vec(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_bootparameters(
        &self,
        auth_token: &str,
        nodes: &[String],
    ) -> Result<Vec<FrontEndBootParameters>, Error> {
        let boot_parameter_vec =
            bss::http_client::get_multiple(auth_token, &self.base_url, &self.root_cert, nodes)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;

        let mut boot_parameter_infra_vec = vec![];

        for boot_parameter in boot_parameter_vec {
            boot_parameter_infra_vec.push(FrontEndBootParameters {
                hosts: boot_parameter.hosts,
                macs: boot_parameter.macs,
                nids: boot_parameter.nids,
                params: boot_parameter.params,
                kernel: boot_parameter.kernel,
                initrd: boot_parameter.initrd,
                cloud_init: boot_parameter.cloud_init,
            });
        }

        Ok(boot_parameter_infra_vec)
    }

    async fn update_bootparameters(
        &self,
        auth_token: &str,
        boot_parameter: &FrontEndBootParameters,
    ) -> Result<(), Error> {
        let boot_parameters = bss::r#struct::BootParameters {
            hosts: boot_parameter.hosts.clone(),
            macs: boot_parameter.macs.clone(),
            nids: boot_parameter.nids.clone(),
            params: boot_parameter.params.clone(),
            kernel: boot_parameter.kernel.clone(),
            initrd: boot_parameter.initrd.clone(),
            cloud_init: boot_parameter.cloud_init.clone(),
        };

        bss::http_client::patch(
            &self.base_url,
            auth_token,
            &self.root_cert,
            &boot_parameters,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_group_map_and_filter_by_group_vec(
        &self,
        auth_token: &str,
        hsm_name_vec: Vec<&str>,
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        hsm::group::utils::get_hsm_map_and_filter_by_hsm_name_vec(
            auth_token,
            &self.base_url,
            &self.root_cert,
            hsm_name_vec,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}
