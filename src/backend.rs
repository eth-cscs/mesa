use infra::{
    self,
    contracts::BackendTrait,
    error::Error,
    types::{BootParameters, HsmGroup, Member},
};
use serde_json::Value;

use crate::{bss, common::authentication, hsm, pcs};

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
        let keycloak_base_url = self.base_url.clone() + "/keycloak";

        authentication::get_api_token(
            &self.base_url,
            &self.root_cert,
            &keycloak_base_url,
            site_name,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_hsm_name_available(&self, auth_token: &str) -> Result<Vec<String>, Error> {
        // Get HSM groups/Keycloak roles the user has access to from JWT token
        let mut realm_access_role_vec = crate::common::jwt_ops::get_hsm_name_available(auth_token)?;

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
            let all_hsm_groups_rslt = self.get_all_hsm(auth_token).await;

            let mut all_hsm_groups = all_hsm_groups_rslt?
                .iter()
                .map(|hsm_value| hsm_value.label.clone())
                .collect::<Vec<String>>();

            all_hsm_groups.sort();

            Ok(all_hsm_groups)
        }
    }

    // FIXME: rename function to 'get_hsm_group_members'
    async fn get_member_vec_from_hsm_name_vec(
        &self,
        auth_token: &str,
        hsm_group_name_vec: Vec<String>,
    ) -> Result<Vec<String>, Error> {
        crate::hsm::group::utils::get_member_vec_from_hsm_name_vec_2(
            auth_token,
            &self.base_url,
            &self.root_cert,
            hsm_group_name_vec,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_all_hsm(&self, auth_token: &str) -> Result<Vec<HsmGroup>, Error> {
        // Get all HSM groups
        let hsm_group_backend_vec =
            crate::hsm::group::http_client::get_all(auth_token, &self.base_url, &self.root_cert)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;

        // Convert all HSM groups from mesa to infra
        let mut hsm_group_vec = Vec::new();

        for hsm_group_backend in hsm_group_backend_vec {
            let mut member_vec = Vec::new();
            let member_vec_backend = hsm_group_backend.members.unwrap().ids.unwrap();

            for member in member_vec_backend {
                member_vec.push(member);
            }

            let members = Member {
                ids: Some(member_vec),
            };

            let hsm_group = HsmGroup {
                label: hsm_group_backend.label,
                description: hsm_group_backend.description,
                tags: hsm_group_backend.tags,
                members: Some(members),
                exclusive_group: hsm_group_backend.exclusive_group,
            };

            hsm_group_vec.push(hsm_group);
        }

        Ok(hsm_group_vec)
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
    ) -> Result<Vec<BootParameters>, Error> {
        let boot_parameter_vec =
            bss::http_client::get_multiple(auth_token, &self.base_url, &self.root_cert, nodes)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;

        let mut boot_parameter_infra_vec = vec![];

        for boot_parameter in boot_parameter_vec {
            boot_parameter_infra_vec.push(BootParameters {
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
        boot_parameter: &BootParameters,
    ) -> Result<BootParameters, Error> {
        let boot_parameters = bss::r#struct::BootParameters {
            hosts: boot_parameter.hosts.clone(),
            macs: boot_parameter.macs.clone(),
            nids: boot_parameter.nids.clone(),
            params: boot_parameter.params.clone(),
            kernel: boot_parameter.kernel.clone(),
            initrd: boot_parameter.initrd.clone(),
            cloud_init: boot_parameter.cloud_init.clone(),
        };

        bss::http_client::put(&self.base_url, auth_token, &self.root_cert, boot_parameters)
            .await
            .map_err(|e| Error::Message(e.to_string()))
            .map(|bp| BootParameters {
                hosts: bp.hosts,
                macs: bp.macs,
                nids: bp.nids,
                params: bp.params,
                kernel: bp.kernel,
                initrd: bp.initrd,
                cloud_init: bp.cloud_init,
            })
    }
}
