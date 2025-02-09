use std::collections::HashMap;

use backend_dispatcher::{
    contracts::BackendTrait,
    error::Error,
    interfaces::{
        bss::BootParametersTrait,
        hsm::{
            component::ComponentTrait, group::GroupTrait, hardware_inventory::HardwareInventory,
        },
        pcs::PCSTrait,
    },
    types::{
        BootParameters as FrontEndBootParameters, Component,
        ComponentArrayPostArray as FrontEndComponentArrayPostArray, Group as FrontEndGroup,
        HWInventoryByLocationList as FrontEndHWInventoryByLocationList, HardwareMetadataArray,
    },
};
use hostlist_parser::parse;
use regex::Regex;
use serde_json::Value;

use crate::{
    bss::{self},
    common::authentication,
    hsm::{self, component::types::ComponentArrayPostArray, group::types::Member},
    pcs,
};

#[derive(Clone)]
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

impl GroupTrait for Csm {
    async fn get_group_available(&self, auth_token: &str) -> Result<Vec<FrontEndGroup>, Error> {
        let mut group_vec = self
            .get_all_groups(auth_token)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        let available_groups_name = self.get_group_name_available(auth_token).await?;

        group_vec.retain(|group| available_groups_name.contains(&group.label));

        Ok(group_vec)
    }

    async fn get_group_name_available(&self, auth_token: &str) -> Result<Vec<String>, Error> {
        log::debug!("Get HSM names available from JWT or all");

        const ADMIN_ROLE_NAME: &str = "pa_admin";

        // Get HSM groups/Keycloak roles the user has access to from JWT token
        let mut realm_access_role_vec = crate::common::jwt_ops::get_roles(auth_token);

        if !realm_access_role_vec.contains(&ADMIN_ROLE_NAME.to_string()) {
            log::debug!("User is not admin, getting HSM groups available from JWT");

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

            Ok(realm_access_role_vec)
        } else {
            log::debug!("User is admin, getting all HSM groups in the system");
            let all_hsm_groups_rslt = self.get_all_groups(auth_token).await;

            let mut all_hsm_groups = all_hsm_groups_rslt?
                .iter()
                .map(|hsm_value| hsm_value.label.clone())
                .collect::<Vec<String>>();

            all_hsm_groups.sort();

            Ok(all_hsm_groups)
        }
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

    async fn delete_group(&self, auth_token: &str, label: &str) -> Result<Value, Error> {
        hsm::group::http_client::delete_group(
            auth_token,
            &self.base_url,
            &self.root_cert,
            &label.to_string(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_hsm_map_and_filter_by_hsm_name_vec(
        &self,
        shasta_token: &str,
        hsm_name_vec: Vec<&str>,
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        hsm::group::utils::get_hsm_map_and_filter_by_hsm_name_vec(
            shasta_token,
            &self.base_url,
            &self.root_cert,
            hsm_name_vec,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn post_member(
        &self,
        auth_token: &str,
        group_label: &str,
        xname: &str,
    ) -> Result<Value, Error> {
        let member = Member {
            id: Some(xname.to_string()),
        };

        hsm::group::http_client::post_member(
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
        new_members: Vec<&str>,
    ) -> Result<Vec<String>, Error> {
        let mut sol: Vec<String> = Vec::new();

        for new_member in new_members {
            sol = hsm::group::utils::add_member(
                auth_token,
                &self.base_url,
                &self.root_cert,
                group_label,
                new_member,
            )
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        }

        Ok(sol)
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
}

impl HardwareInventory for Csm {
    async fn get_inventory_hardware_query(
        &self,
        auth_token: &str,
        xname: &str,
        r#_type: Option<&str>,
        _children: Option<bool>,
        _parents: Option<bool>,
        _partition: Option<&str>,
        _format: Option<&str>,
    ) -> Result<Value, Error> {
        hsm::hw_inventory::hw_component::http_client::get_hw_inventory(
            &auth_token,
            &self.base_url,
            &self.root_cert,
            xname,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn post_inventory_hardware(
        &self,
        auth_token: &str,
        hw_inventory: FrontEndHWInventoryByLocationList,
    ) -> Result<Value, Error> {
        hsm::hw_inventory::hw_component::http_client::post(
            auth_token,
            &self.base_url,
            &self.root_cert,
            hw_inventory.into(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl ComponentTrait for Csm {
    async fn get_all_nodes(
        &self,
        auth_token: &str,
        nid_only: Option<&str>,
    ) -> Result<HardwareMetadataArray, Error> {
        hsm::component::http_client::get(
            &self.base_url,
            &self.root_cert,
            auth_token,
            None,
            Some("Node"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            nid_only,
        )
        .await
        .map(|c| c.into())
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_node_metadata_available(&self, auth_token: &str) -> Result<Vec<Component>, Error> {
        let xname_available_vec: Vec<String> = self
            .get_group_available(auth_token)
            .await
            .map_err(|e| Error::Message(e.to_string()))?
            .iter()
            .flat_map(|group| group.get_members())
            .collect();

        let node_metadata_vec: Vec<Component> = self
            .get_all_nodes(auth_token, Some("true"))
            .await
            .unwrap()
            .components
            .unwrap_or_default()
            .iter()
            .filter(|&node_metadata| {
                xname_available_vec.contains(&node_metadata.id.as_ref().unwrap())
            })
            .cloned()
            .collect();

        Ok(node_metadata_vec)
    }

    async fn get(
        &self,
        auth_token: &str,
        id: Option<&str>,
        r#type: Option<&str>,
        state: Option<&str>,
        flag: Option<&str>,
        role: Option<&str>,
        subrole: Option<&str>,
        enabled: Option<&str>,
        software_status: Option<&str>,
        subtype: Option<&str>,
        arch: Option<&str>,
        class: Option<&str>,
        nid: Option<&str>,
        nid_start: Option<&str>,
        nid_end: Option<&str>,
        partition: Option<&str>,
        group: Option<&str>,
        state_only: Option<&str>,
        flag_only: Option<&str>,
        role_only: Option<&str>,
        nid_only: Option<&str>,
    ) -> Result<HardwareMetadataArray, Error> {
        hsm::component::http_client::get(
            &self.base_url,
            &self.root_cert,
            auth_token,
            id,
            r#type,
            state,
            flag,
            role,
            subrole,
            enabled,
            software_status,
            subtype,
            arch,
            class,
            nid,
            nid_start,
            nid_end,
            partition,
            group,
            state_only,
            flag_only,
            role_only,
            nid_only,
        )
        .await
        .map(|c| c.into())
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn post_nodes(
        &self,
        auth_token: &str,
        component: FrontEndComponentArrayPostArray,
    ) -> Result<(), Error> {
        let component_backend: ComponentArrayPostArray = component.into();

        hsm::component::http_client::post(
            auth_token,
            &self.base_url,
            &self.root_cert,
            component_backend,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn delete_node(&self, auth_token: &str, id: &str) -> Result<Value, Error> {
        hsm::component::http_client::delete_one(auth_token, &self.base_url, &self.root_cert, id)
            .await
            .map_err(|e| Error::Message(e.to_string()))
    }
}

impl PCSTrait for Csm {
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
}

impl BootParametersTrait for Csm {
    async fn get_bootparameters(
        &self,
        auth_token: &str,
        nodes: &[String],
    ) -> Result<Vec<FrontEndBootParameters>, Error> {
        let boot_parameter_vec =
            bss::http_client::get_multiple(auth_token, &self.base_url, &self.root_cert, nodes)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;

        let boot_parameter_infra_vec = boot_parameter_vec
            .into_iter()
            .map(|boot_parameter| boot_parameter.into())
            .collect();

        Ok(boot_parameter_infra_vec)
    }

    async fn update_bootparameters(
        &self,
        auth_token: &str,
        boot_parameter: &FrontEndBootParameters,
    ) -> Result<(), Error> {
        let boot_parameters = bss::types::BootParameters {
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

    /// Get list of xnames from NIDs
    /// The list of NIDs can be:
    ///     - comma separated list of NIDs (eg: nid000001,nid000002,nid000003)
    ///     - regex (eg: nid00000.*)
    ///     - hostlist (eg: nid0000[01-15])
    async fn nid_to_xname(
        &self,
        shasta_token: &str,
        user_input_nid: &str,
        is_regex: bool,
    ) -> Result<Vec<String>, Error> {
        if is_regex {
            log::debug!("Regex found, getting xnames from NIDs");
            // Get list of regex
            let regex_vec: Vec<Regex> = user_input_nid
                .split(",")
                .map(|regex_str| Regex::new(regex_str.trim()))
                .collect::<Result<Vec<Regex>, regex::Error>>()
                .map_err(|e| Error::Message(e.to_string()))?;

            // Get all HSM components (list of xnames + nids)
            let hsm_component_vec = hsm::component::http_client::get_all_nodes(
                &self.base_url,
                &self.root_cert,
                shasta_token,
                Some("true"),
            )
            .await
            .map_err(|e| Error::Message(e.to_string()))?
            .components
            .unwrap_or_default();

            let mut xname_vec: Vec<String> = vec![];

            // Get list of xnames the user is asking for
            for hsm_component in hsm_component_vec {
                let nid_long = format!("nid{:06}", &hsm_component.nid.expect("No NID found"));
                for regex in &regex_vec {
                    if regex.is_match(&nid_long) {
                        log::debug!(
                            "Nid '{}' IS included in regex '{}'",
                            nid_long,
                            regex.as_str()
                        );
                        xname_vec.push(hsm_component.id.clone().expect("No XName found"));
                    }
                }
            }

            return Ok(xname_vec);
        } else {
            log::debug!("No regex found, getting xnames from list of NIDs or NIDs hostlist");
            let nid_hostlist_expanded_vec = parse(user_input_nid).map_err(|e| {
                Error::Message(format!(
                    "Could not parse list of nodes as a hostlist. Reason:\n{}Exit",
                    e
                ))
            })?;

            log::debug!("hostlist: {}", user_input_nid);
            log::debug!("hostlist expanded: {:?}", nid_hostlist_expanded_vec);

            let nid_short = nid_hostlist_expanded_vec
                .iter()
                .map(|nid_long| {
                    nid_long
                        .strip_prefix("nid")
                        .expect(
                            format!("Nid '{}' not valid, 'nid' prefix missing", nid_long).as_str(),
                        )
                        .trim_start_matches("0")
                })
                .collect::<Vec<&str>>()
                .join(",");

            log::debug!("short NID list: {}", nid_short);

            let hsm_components = hsm::component::http_client::get(
                &self.base_url,
                &self.root_cert,
                shasta_token,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&nid_short),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some("true"),
            )
            .await
            .map_err(|e| Error::Message(e.to_string()))?;

            // Get list of xnames from HSM components
            let xname_vec: Vec<String> = hsm_components
                .components
                .unwrap_or_default()
                .iter()
                .map(|component| component.id.clone().unwrap())
                .collect();

            log::debug!("xname list:\n{:#?}", xname_vec);

            return Ok(xname_vec);
        };
    }
}
