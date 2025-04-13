use std::{collections::HashMap, path::PathBuf, pin::Pin};

use backend_dispatcher::{
    contracts::BackendTrait,
    error::Error,
    interfaces::{
        apply_hw_cluster_pin::ApplyHwClusterPin,
        apply_session::ApplySessionTrait,
        bss::BootParametersTrait,
        cfs::CfsTrait,
        get_bos_session_templates::GetTemplatesTrait,
        get_images_and_details::GetImagesAndDetailsTrait,
        hsm::{
            component::ComponentTrait, group::GroupTrait, hardware_inventory::HardwareInventory,
            redfish_endpoint::RedfishEndpointTrait,
        },
        ims::ImsTrait,
        migrate_backup::MigrateBackupTrait,
        migrate_restore::MigrateRestoreTrait,
        pcs::PCSTrait,
        sat::SatTrait,
    },
    types::{
        cfs::{
            cfs_configuration_request::CfsConfigurationRequest, CfsConfigurationResponse,
            CfsSessionGetResponse, CfsSessionPostRequest, Layer, LayerDetails,
        },
        hsm::inventory::RedfishEndpointArray as FrontEndRedfishEndpointArray,
        ims::Image as FrontEndImage,
        BootParameters as FrontEndBootParameters, BosSessionTemplate,
        BosSessionTemplate as FrontEndBosSessionTemplate, Component,
        ComponentArrayPostArray as FrontEndComponentArrayPostArray, Group as FrontEndGroup,
        HWInventoryByLocationList as FrontEndHWInventoryByLocationList, K8sAuth, K8sDetails,
        NodeMetadataArray,
    },
};
use futures::{AsyncBufRead, AsyncReadExt};
use hostlist_parser::parse;
use regex::Regex;
use serde_json::Value;

use crate::{
    bos,
    bss::{self},
    common::{authentication, kubernetes, vault::http_client::fetch_shasta_k8s_secrets_from_vault},
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
        hsm::group::utils::get_group_name_available(auth_token, &self.base_url, &self.root_cert)
            .await
            .map_err(|e| Error::Message(e.to_string()))
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
            group.clone().into(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        // let group: FrontEndGroup = group_csm.into();
        log::info!("Group created: {}", group_csm);

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

    async fn get_group_map_and_filter_by_member_vec(
        &self,
        auth_token: &str,
        member_vec: &[&str],
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        hsm::group::utils::get_hsm_group_map_and_filter_by_hsm_group_member_vec(
            auth_token,
            &self.base_url,
            &self.root_cert,
            member_vec,
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
            Some(&[hsm_name]),
            None,
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

    async fn get_groups(
        &self,
        auth_token: &str,
        hsm_name_vec: Option<&[&str]>,
    ) -> Result<Vec<FrontEndGroup>, Error> {
        // Get all HSM groups
        let hsm_group_backend_vec = hsm::group::http_client::get(
            auth_token,
            &self.base_url,
            &self.root_cert,
            hsm_name_vec,
            None,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        // Convert from HsmGroup (silla) to HsmGroup (infra)
        let mut hsm_group_vec = Vec::new();
        for hsm_group_backend in hsm_group_backend_vec {
            let hsm_group: FrontEndGroup = hsm_group_backend.into();
            hsm_group_vec.push(hsm_group);
        }

        Ok(hsm_group_vec)
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
    async fn get_inventory_hardware(&self, auth_token: &str, xname: &str) -> Result<Value, Error> {
        hsm::hw_inventory::hw_component::http_client::get(
            auth_token,
            &self.base_url,
            &self.root_cert,
            xname,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
        .and_then(|hw_inventory| {
            serde_json::to_value(hw_inventory).map_err(|e| Error::Message(e.to_string()))
        })
    }

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
        hsm::hw_inventory::hw_component::http_client::get_query(
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
    ) -> Result<NodeMetadataArray, Error> {
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

        let node_metadata_vec_rslt = self
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

        let node_metadata_vec: Vec<Component> = node_metadata_vec_rslt;

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
    ) -> Result<NodeMetadataArray, Error> {
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

    async fn add_bootparameters(
        &self,
        auth_token: &str,
        boot_parameters: &FrontEndBootParameters,
    ) -> Result<(), Error> {
        bss::http_client::post(
            &self.base_url,
            auth_token,
            &self.root_cert,
            boot_parameters.clone().into(),
        )
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn update_bootparameters(
        &self,
        auth_token: &str,
        boot_parameter: &FrontEndBootParameters,
    ) -> Result<(), Error> {
        bss::http_client::patch(
            &self.base_url,
            auth_token,
            &self.root_cert,
            &boot_parameter.clone().into(),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn delete_bootparameters(
        &self,
        _auth_token: &str,
        _boot_parameters: &FrontEndBootParameters,
    ) -> Result<String, Error> {
        Err(Error::Message(
            "Delete boot parameters command not implemented for this backend".to_string(),
        ))
    }
}

impl RedfishEndpointTrait for Csm {
    async fn get_redfish_endpoints(
        &self,
        _auth_token: &str,
        _id: Option<&str>,
        _fqdn: Option<&str>,
        _type: Option<&str>,
        _uuid: Option<&str>,
        _macaddr: Option<&str>,
        _ip_address: Option<&str>,
        _last_status: Option<&str>,
    ) -> Result<FrontEndRedfishEndpointArray, Error> {
        Err(Error::Message(
            "Get redfish endpoint command not implemented for this backend".to_string(),
        ))
    }

    async fn add_redfish_endpoint(
        &self,
        _auth_token: &str,
        _redfish_endpoint: &backend_dispatcher::types::hsm::inventory::RedfishEndpoint,
    ) -> Result<(), Error> {
        Err(Error::Message(
            "Add redfish endpoint command not implemented for this backend".to_string(),
        ))
    }

    async fn update_redfish_endpoint(
        &self,
        _auth_token: &str,
        _redfish_endpoint: &backend_dispatcher::types::hsm::inventory::RedfishEndpoint,
    ) -> Result<(), Error> {
        Err(Error::Message(
            "Update redfish endpoint command not implemented for this backend".to_string(),
        ))
    }

    async fn delete_redfish_endpoint(&self, _auth_token: &str, _id: &str) -> Result<Value, Error> {
        Err(Error::Message(
            "Delete redfish endpoint command not implemented for this backend".to_string(),
        ))
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
                let nid_long = format!(
                    "nid{:06}",
                    &hsm_component
                        .nid
                        .ok_or_else(|| Error::Message("No NID found".to_string()))?
                );
                for regex in &regex_vec {
                    if regex.is_match(&nid_long) {
                        log::debug!(
                            "Nid '{}' IS included in regex '{}'",
                            nid_long,
                            regex.as_str()
                        );
                        xname_vec.push(
                            hsm_component
                                .id
                                .clone()
                                .ok_or_else(|| Error::Message("No XName found".to_string()))?,
                        );
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

            let mut nid_short_vec = Vec::new();

            for nid_long in nid_hostlist_expanded_vec {
                let nid_short_elem = nid_long
                    .strip_prefix("nid")
                    .ok_or_else(|| {
                        Error::Message(format!(
                            "Nid '{}' not valid, 'nid' prefix missing",
                            nid_long
                        ))
                    })?
                    .trim_start_matches("0");

                nid_short_vec.push(nid_short_elem.to_string());
            }

            let nid_short = nid_short_vec.join(",");
            /* let nid_short = nid_hostlist_expanded_vec
            .iter()
            .map(|nid_long| {
                nid_long
                    .strip_prefix("nid")
                    .ok_or_else(|| {
                        Error::Message(format!(
                            "Nid '{}' not valid, 'nid' prefix missing",
                            nid_long
                        ))
                    })?
                    /* .expect(
                        format!("Nid '{}' not valid, 'nid' prefix missing", nid_long).as_str(),
                    ) */
                    .trim_start_matches("0")
            })
            .collect::<Vec<&str>>()
            .join(","); */

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

impl CfsTrait for Csm {
    type T = Pin<Box<dyn AsyncBufRead + Send>>;

    async fn get_session_logs_stream(
        &self,
        shasta_token: &str,
        site_name: &str,
        cfs_session_name: &str,
        // k8s_api_url: &str,
        k8s: &K8sDetails,
    ) -> Result<Pin<Box<dyn AsyncBufRead + Send>>, Error> {
        // FIXME: this only takes the stream from the CFS session and not from the
        // git-clone init container
        let shasta_k8s_secrets = match &k8s.authentication {
            K8sAuth::Native {
                certificate_authority_data,
                client_certificate_data,
                client_key_data,
            } => {
                serde_json::json!({ "certificate-authority-data": certificate_authority_data, "client-certificate-data": client_certificate_data, "client-key-data": client_key_data })
            }
            K8sAuth::Vault { base_url } => {
                fetch_shasta_k8s_secrets_from_vault(&base_url, shasta_token, &site_name)
                    .await
                    .map_err(|e| Error::Message(format!("{e}")))?
            }
        };

        let client = kubernetes::get_k8s_client_programmatically(&k8s.api_url, shasta_k8s_secrets)
            .await
            .map_err(|e| Error::Message(format!("{e}")))?;

        let log_stream_git_clone =
            kubernetes::get_cfs_session_init_container_git_clone_logs_stream(
                client.clone(),
                cfs_session_name,
            )
            .await
            .map_err(|e| Error::Message(format!("{e}")))?;

        let log_stream_inventory = kubernetes::get_cfs_session_container_inventory_logs_stream(
            client.clone(),
            cfs_session_name,
        )
        .await
        .map_err(|e| Error::Message(format!("{e}")))?;

        let log_stream_ansible =
            kubernetes::get_cfs_session_container_ansible_logs_stream(client, cfs_session_name)
                .await
                .map_err(|e| Error::Message(format!("{e}")))?;

        // NOTE: here is where we convert from impl AsyncBufRead to Pin<Box<dyn AsyncBufRead>>
        // through dynamic dispatch
        Ok(Box::pin(
            log_stream_git_clone
                .chain(log_stream_inventory)
                .chain(log_stream_ansible),
        ))
    }

    async fn get_session_logs_stream_by_xname(
        &self,
        auth_token: &str,
        site_name: &str,
        xname: &str,
        // k8s_api_url: &str,
        k8s: &K8sDetails,
    ) -> Result<Pin<Box<dyn AsyncBufRead + Send>>, Error> {
        let mut session_vec = crate::cfs::session::http_client::v3::get(
            auth_token,
            self.base_url.as_str(),
            self.root_cert.as_slice(),
            None,
            Some(1),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        crate::cfs::session::utils::filter_by_xname(
            auth_token,
            &self.base_url,
            &self.root_cert,
            &mut session_vec,
            &[xname],
            None,
            true,
        )
        .await;

        if session_vec.is_empty() {
            return Err(Error::Message(format!(
                "No CFS session found for xname '{}'",
                xname
            )));
        }

        self.get_session_logs_stream(
            auth_token,
            site_name,
            session_vec.first().unwrap().name.as_ref().unwrap().as_str(),
            // k8s_api_url,
            k8s,
        )
        .await
    }

    async fn post_session(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        session: &CfsSessionPostRequest,
    ) -> Result<CfsSessionGetResponse, Error> {
        crate::cfs::session::http_client::v3::post(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &session.clone().into(),
        )
        .await
        .map(|cfs_session| cfs_session.into())
        .map_err(|e| Error::Message(e.to_string()))
    }

    /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
    async fn get_sessions(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        session_name_opt: Option<&String>,
        limit_opt: Option<u8>,
        after_id_opt: Option<String>,
        min_age_opt: Option<String>,
        max_age_opt: Option<String>,
        status_opt: Option<String>,
        name_contains_opt: Option<String>,
        is_succeded_opt: Option<bool>,
        tags_opt: Option<String>,
    ) -> Result<Vec<CfsSessionGetResponse>, Error> {
        // Get local/backend CFS sessions
        let local_cfs_session_vec = crate::cfs::session::http_client::v3::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            session_name_opt,
            limit_opt,
            after_id_opt,
            min_age_opt,
            max_age_opt,
            status_opt,
            name_contains_opt,
            is_succeded_opt,
            tags_opt,
        )
        .await;

        // Convert to manta session
        let border_session_vec = local_cfs_session_vec
            .map(|cfs_session_vec| {
                cfs_session_vec
                    .into_iter()
                    .map(|cfs_session| cfs_session.into())
                    .collect::<Vec<CfsSessionGetResponse>>()
            })
            .map_err(|e| Error::Message(e.to_string()));

        border_session_vec
    }

    async fn get_and_filter_sessions(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec_opt: Option<Vec<String>>,
        xname_vec_opt: Option<Vec<&str>>,
        min_age_opt: Option<&String>,
        max_age_opt: Option<&String>,
        status_opt: Option<&String>,
        cfs_session_name_opt: Option<&String>,
        limit_number_opt: Option<&u8>,
        is_succeded_opt: Option<bool>,
    ) -> Result<Vec<CfsSessionGetResponse>, Error> {
        let mut cfs_session_vec = crate::cfs::session::get_and_sort(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            min_age_opt,
            max_age_opt,
            status_opt,
            cfs_session_name_opt,
            is_succeded_opt,
        )
        .await
        .unwrap();

        if let Some(hsm_group_name_vec) = hsm_group_name_vec_opt {
            if !hsm_group_name_vec.is_empty() {
                crate::cfs::session::utils::filter_by_hsm(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    &mut cfs_session_vec,
                    &hsm_group_name_vec,
                    limit_number_opt,
                    true,
                )
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
            }
        }

        if let Some(xname_vec) = xname_vec_opt {
            crate::cfs::session::utils::filter_by_xname(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                &mut cfs_session_vec,
                xname_vec.as_slice(),
                limit_number_opt,
                true,
            )
            .await;
        }

        if cfs_session_vec.is_empty() {
            return Err(Error::Message("No CFS session found".to_string()));
        }

        for cfs_session in cfs_session_vec.iter_mut() {
            log::debug!("CFS session:\n{:#?}", cfs_session);

            if cfs_session
                .target
                .as_ref()
                .unwrap()
                .definition
                .as_ref()
                .unwrap()
                .eq("image")
                && cfs_session
                    .status
                    .as_ref()
                    .unwrap()
                    .session
                    .as_ref()
                    .unwrap()
                    .succeeded
                    .as_ref()
                    .unwrap()
                    .eq("true")
            {
                log::info!(
                    "Find image ID related to CFS configuration {} in CFS session {}",
                    cfs_session
                        .configuration
                        .as_ref()
                        .unwrap()
                        .name
                        .as_ref()
                        .unwrap(),
                    cfs_session.name.as_ref().unwrap()
                );

                let new_image_id_opt = if cfs_session
                    .status
                    .as_ref()
                    .and_then(|status| {
                        status.artifacts.as_ref().and_then(|artifacts| {
                            artifacts
                                .first()
                                .and_then(|artifact| artifact.result_id.clone())
                        })
                    })
                    .is_some()
                {
                    let cfs_session_image_id = cfs_session
                        .status
                        .as_ref()
                        .unwrap()
                        .artifacts
                        .as_ref()
                        .unwrap()
                        .first()
                        .unwrap()
                        .result_id
                        .as_ref();

                    let image_id = cfs_session_image_id.map(|elem| elem.as_str());

                    let new_image_vec_rslt: Result<
                        Vec<crate::ims::image::http_client::types::Image>,
                        _,
                    > = crate::ims::image::http_client::get(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        // hsm_group_name_vec,
                        image_id,
                    )
                    .await;

                    // if new_image_id_vec_rslt.is_ok() && new_image_id_vec_rslt.as_ref().unwrap().first().is_some()
                    if let Ok(Some(new_image)) = new_image_vec_rslt
                        .as_ref()
                        .map(|new_image_vec| new_image_vec.first())
                    {
                        Some(new_image.clone().id.unwrap_or("".to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if new_image_id_opt.is_some() {
                    cfs_session
                        .status
                        .clone()
                        .unwrap()
                        .artifacts
                        .unwrap()
                        .first()
                        .unwrap()
                        .clone()
                        .result_id = new_image_id_opt;
                }
            }
        }

        Ok(cfs_session_vec
            .into_iter()
            .map(|cfs_session| cfs_session.into())
            .collect())
    }

    async fn create_configuration_from_repos(
        &self,
        gitea_token: &str,
        gitea_base_url: &str,
        shasta_root_cert: &[u8],
        repo_name_vec: Vec<String>,
        local_git_commit_vec: Vec<String>,
        playbook_file_name_opt: Option<&String>,
    ) -> Result<CfsConfigurationRequest, Error> {
        Ok(crate::cfs::configuration::http_client::v3::types::cfs_configuration_request::CfsConfigurationRequest::create_from_repos(
            gitea_token,
            gitea_base_url,
            shasta_root_cert,
            repo_name_vec,
            local_git_commit_vec,
            playbook_file_name_opt,
        ).await.map_err(|e| Error::Message(e.to_string()))?.into())
    }

    async fn get_configuration(
        &self,
        auth_token: &str,
        base_url: &str,
        root_cert: &[u8],
        configuration_name_opt: Option<&String>,
    ) -> Result<Vec<CfsConfigurationResponse>, Error> {
        let cfs_configuration_vec = crate::cfs::configuration::http_client::v3::get(
            auth_token,
            base_url,
            root_cert,
            configuration_name_opt.map(|elem| elem.as_str()),
        )
        .await
        .map_err(|e| Error::Message(e.to_string()));

        cfs_configuration_vec.map(|config_vec| config_vec.into_iter().map(|c| c.into()).collect())
    }

    async fn get_and_filter_configuration(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration_name: Option<&str>,
        configuration_name_pattern: Option<&str>,
        hsm_group_name_vec: &[String],
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<CfsConfigurationResponse>, Error> {
        crate::cfs::configuration::utils::get_and_filter(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            configuration_name,
            configuration_name_pattern,
            hsm_group_name_vec,
            limit_number_opt,
        )
        .await
        .map(|config_vec| config_vec.into_iter().map(|c| c.into()).collect())
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn get_configuration_layer_details(
        &self,
        shasta_root_cert: &[u8],
        gitea_base_url: &str,
        gitea_token: &str,
        layer: Layer,
        site_name: &str,
    ) -> Result<LayerDetails, Error> {
        crate::cfs::configuration::utils::get_configuration_layer_details(
            shasta_root_cert,
            gitea_base_url,
            gitea_token,
            layer.into(),
            site_name,
        )
        .await
        .map(|layer_details| layer_details.into())
        .map_err(|e| Error::Message(e.to_string()))
    }

    async fn put_configuration(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration: &CfsConfigurationRequest,
        configuration_name: &str,
    ) -> Result<CfsConfigurationResponse, Error> {
        crate::cfs::configuration::http_client::v3::put(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &configuration.clone().into(),
            configuration_name,
        )
        .await
        .map(|config| config.into())
        .map_err(|e| Error::Message(e.to_string()))
    }

    /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
    async fn get_sessions_by_xname(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        xname_vec: &[&str],
        limit_opt: Option<u8>,
        after_id_opt: Option<String>,
        min_age_opt: Option<String>,
        max_age_opt: Option<String>,
        status_opt: Option<String>,
        name_contains_opt: Option<String>,
        is_succeded_opt: Option<bool>,
        tags_opt: Option<String>,
    ) -> Result<Vec<CfsSessionGetResponse>, Error> {
        // Get local/backend CFS sessions
        let mut local_cfs_session_vec = crate::cfs::session::http_client::v3::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            None,
            limit_opt,
            after_id_opt,
            min_age_opt,
            max_age_opt,
            status_opt,
            name_contains_opt,
            is_succeded_opt,
            tags_opt,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

        crate::cfs::session::utils::filter_by_xname(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            &mut local_cfs_session_vec,
            xname_vec,
            None,
            true,
        )
        .await;

        // Convert to manta session
        let border_session_vec = local_cfs_session_vec
            .into_iter()
            .map(|cfs_session| cfs_session.into())
            .collect::<Vec<CfsSessionGetResponse>>();

        Ok(border_session_vec)
    }

    async fn update_runtime_configuration(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        xnames: Vec<String>,
        desired_configuration: &str,
        enabled: bool,
    ) -> Result<(), Error> {
        crate::cfs::component::utils::update_component_list_desired_configuration(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            xnames,
            desired_configuration,
            enabled,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }

    // Get all CFS sessions, IMS images and BOS sessiontemplates related to a CFS configuration
    async fn get_derivatives(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        configuration_name: &str,
    ) -> Result<
        (
            Option<Vec<CfsSessionGetResponse>>,
            Option<Vec<BosSessionTemplate>>,
            Option<Vec<FrontEndImage>>,
        ),
        Error,
    > {
        crate::cfs::configuration::utils::get_derivatives(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            configuration_name,
        )
        .await
        .map(|(cfs_session_vec, bos_session_template_vec, image_vec)| {
            (
                cfs_session_vec.map(|cfs_session_vec| {
                    cfs_session_vec
                        .into_iter()
                        .map(|cfs_session| cfs_session.into())
                        .collect()
                }),
                bos_session_template_vec.map(|bos_session_template_vec| {
                    bos_session_template_vec
                        .into_iter()
                        .map(|bos_session_template| bos_session_template.into())
                        .collect()
                }),
                image_vec
                    .map(|image_vec| image_vec.into_iter().map(|image| image.into()).collect()),
            )
        })
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl SatTrait for Csm {
    async fn apply_sat_file(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        vault_base_url: &str,
        site_name: &str,
        // vault_secret_path: &str,
        // vault_role_id: &str,
        k8s_api_url: &str,
        shasta_k8s_secrets: serde_json::Value,
        // sat_file_content: String,
        sat_template_file_yaml: serde_yaml::Value,
        hsm_group_param_opt: Option<&String>,
        hsm_group_available_vec: &Vec<String>,
        ansible_verbosity_opt: Option<u8>,
        ansible_passthrough_opt: Option<&String>,
        gitea_base_url: &str,
        gitea_token: &str,
        do_not_reboot: bool,
        watch_logs: bool,
        image_only: bool,
        session_template_only: bool,
        debug_on_failure: bool,
        dry_run: bool,
    ) -> Result<(), Error> {
        crate::commands::apply_sat_file::command::exec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            vault_base_url,
            site_name,
            // vault_secret_path,
            // vault_role_id,
            k8s_api_url,
            shasta_k8s_secrets,
            // sat_file_content,
            sat_template_file_yaml,
            hsm_group_param_opt,
            hsm_group_available_vec,
            ansible_verbosity_opt,
            ansible_passthrough_opt,
            gitea_base_url,
            gitea_token,
            do_not_reboot,
            watch_logs,
            image_only,
            session_template_only,
            debug_on_failure,
            dry_run,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl ApplyHwClusterPin for Csm {
    async fn apply_hw_cluster_pin(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        target_hsm_group_name: &str,
        parent_hsm_group_name: &str,
        pattern: &str,
        nodryrun: bool,
        create_target_hsm_group: bool,
        delete_empty_parent_hsm_group: bool,
    ) -> Result<(), Error> {
        crate::commands::apply_hw_cluster_pin::command::exec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            target_hsm_group_name,
            parent_hsm_group_name,
            pattern,
            nodryrun,
            create_target_hsm_group,
            delete_empty_parent_hsm_group,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl ImsTrait for Csm {
    async fn get_images(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        image_id_opt: Option<&str>,
    ) -> Result<Vec<FrontEndImage>, Error> {
        crate::ims::image::http_client::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            image_id_opt,
        )
        .await
        .map(|image_vec| image_vec.into_iter().map(|image| image.into()).collect())
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl ApplySessionTrait for Csm {
    async fn apply_session(
        &self,
        gitea_token: &str,
        gitea_base_url: &str,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        // k8s_api_url: &str,
        cfs_conf_sess_name: Option<&String>,
        playbook_yaml_file_name_opt: Option<&String>,
        hsm_group: Option<&String>,
        repos_paths: Vec<PathBuf>,
        ansible_limit: Option<String>,
        ansible_verbosity: Option<String>,
        ansible_passthrough: Option<String>,
        // watch_logs: bool,
        /* kafka_audit: &Kafka,
        k8s: &K8sDetails, */
    ) -> Result<(String, String), Error> {
        crate::commands::apply_session::exec(
            gitea_token,
            gitea_base_url,
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            // k8s_api_url,
            cfs_conf_sess_name,
            playbook_yaml_file_name_opt,
            hsm_group,
            repos_paths,
            ansible_limit,
            ansible_verbosity,
            ansible_passthrough,
            // watch_logs,
            /* kafka_audit,
            k8s, */
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl MigrateRestoreTrait for Csm {
    async fn migrate_restore(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        bos_file: Option<&String>,
        cfs_file: Option<&String>,
        hsm_file: Option<&String>,
        ims_file: Option<&String>,
        image_dir: Option<&String>,
    ) -> Result<(), Error> {
        crate::commands::migrate_restore::exec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            bos_file,
            cfs_file,
            hsm_file,
            ims_file,
            image_dir,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl MigrateBackupTrait for Csm {
    async fn migrate_backup(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        bos: Option<&String>,
        destination: Option<&String>,
    ) -> Result<(), Error> {
        crate::commands::migrate_backup::exec(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            bos,
            destination,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl GetImagesAndDetailsTrait for Csm {
    async fn get_images_and_details(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec: &[String],
        id_opt: Option<&String>,
        limit_number: Option<&u8>,
    ) -> Result<Vec<(FrontEndImage, String, String, bool)>, Error> {
        crate::commands::get_images_and_details::get_images_and_details(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            hsm_group_name_vec,
            id_opt,
            limit_number,
        )
        .await
        .map(|image_details_vec| {
            image_details_vec
                .into_iter()
                .map(|(image, x, y, z)| (image.into(), x, y, z))
                .collect()
        })
        .map_err(|e| Error::Message(e.to_string()))
    }
}

impl GetTemplatesTrait for Csm {
    async fn get_templates(
        &self,
        shasta_token: &str,
        shasta_base_url: &str,
        shasta_root_cert: &[u8],
        hsm_group_name_vec: &Vec<String>,
        hsm_member_vec: &[String],
        bos_sessiontemplate_name_opt: Option<&String>,
        limit_number_opt: Option<&u8>,
    ) -> Result<Vec<FrontEndBosSessionTemplate>, Error> {
        let bos_sessiontemplate_vec_rslt = bos::template::http_client::v2::get(
            shasta_token,
            shasta_base_url,
            shasta_root_cert,
            bos_sessiontemplate_name_opt.map(|value| value.as_str()),
        )
        .await;

        let mut bos_sessiontemplate_vec = match bos_sessiontemplate_vec_rslt {
            Ok(bos_sessiontemplate_vec) => bos_sessiontemplate_vec,
            Err(e) => {
                eprintln!(
                    "ERROR - Could not fetch BOS sessiontemplate list. Reason:\n{:#?}\nExit",
                    e
                );
                std::process::exit(1);
            }
        };

        bos::template::utils::filter(
            &mut bos_sessiontemplate_vec,
            hsm_group_name_vec,
            hsm_member_vec,
            limit_number_opt,
        );

        Ok(bos_sessiontemplate_vec
            .into_iter()
            .map(|template| template.into())
            .collect::<Vec<FrontEndBosSessionTemplate>>())
    }
}
