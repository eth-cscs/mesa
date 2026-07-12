//! `HardwareInventory`, `ComponentTrait`, `ComponentEthernetInterfaceTrait`, `RedfishEndpointTrait` impls for [`super::Csm`].

use hostlist_parser::parse;
use manta_backend_dispatcher::{
  error::Error,
  interfaces::hsm::{
    component::ComponentTrait,
    component_ethernet_interface::ComponentEthernetInterfaceTrait,
    group::GroupTrait, hardware_inventory::HardwareInventory,
    redfish_endpoint::RedfishEndpointTrait,
  },
  types::{
    Component, ComponentArrayPostArray as FrontEndComponentArrayPostArray,
    HWInventory as FrontEndHWInventory,
    HWInventoryByLocationList as FrontEndHWInventoryByLocationList,
    HsmActionResponse, NodeMetadataArray, NodeSummary as FrontEndNodeSummary,
    hsm::inventory::{
      ComponentEthernetInterface,
      RedfishEndpointArray as FrontEndRedfishEndpointArray,
    },
  },
};
use regex::Regex;
use serde_json::Value;

use super::Csm;
use crate::hsm::component::types::ComponentArrayPostArray;

impl HardwareInventory for Csm {
  async fn get_inventory_hardware(
    &self,
    auth_token: &str,
    xname: &str,
  ) -> Result<FrontEndNodeSummary, Error> {
    self
      .shasta_client()
      .hsm_hw_inventory_get(auth_token, xname)
      .await
      .map(Into::into)
      .map_err(Error::from)
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
  ) -> Result<FrontEndHWInventory, Error> {
    self
      .shasta_client()
      .hsm_hw_inventory_get_query(auth_token, xname)
      .await
      .map(Into::into)
      .map_err(Error::from)
  }

  async fn post_inventory_hardware(
    &self,
    auth_token: &str,
    hw_inventory: FrontEndHWInventoryByLocationList,
  ) -> Result<HsmActionResponse, Error> {
    self
      .shasta_client()
      .hsm_hw_inventory_post(auth_token, hw_inventory.into())
      .await
      .map(Into::into)
      .map_err(Error::from)
  }
}

impl ComponentTrait for Csm {
  async fn get_all_nodes(
    &self,
    auth_token: &str,
    nid_only: Option<&str>,
  ) -> Result<NodeMetadataArray, Error> {
    self
      .shasta_client()
      .hsm_component_get_all_nodes(auth_token, nid_only)
      .await
      .map(std::convert::Into::into)
      .map_err(Error::from)
  }

  async fn get_node_metadata_available(
    &self,
    auth_token: &str,
  ) -> Result<Vec<Component>, Error> {
    let xname_available_vec: Vec<String> = self
      .get_group_available(auth_token)
      .await?
      .iter()
      .flat_map(manta_backend_dispatcher::types::Group::get_members)
      .collect();

    let node_metadata_vec_rslt = self
      .get_all_nodes(auth_token, Some("false"))
      .await?
      .components
      .unwrap_or_default()
      .iter()
      .filter(|&node_metadata| {
        node_metadata
          .id
          .as_ref()
          .is_some_and(|id| xname_available_vec.contains(id))
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
    let _ = role_only;
    self
      .shasta_client()
      .hsm_component_get(
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
        nid_only,
      )
      .await
      .map(std::convert::Into::into)
      .map_err(Error::from)
  }

  async fn post_nodes(
    &self,
    auth_token: &str,
    component: FrontEndComponentArrayPostArray,
  ) -> Result<(), Error> {
    let component_backend: ComponentArrayPostArray = component.into();

    self
      .shasta_client()
      .hsm_component_post(auth_token, component_backend)
      .await
      .map_err(Error::from)
  }

  async fn delete_node(
    &self,
    auth_token: &str,
    id: &str,
  ) -> Result<HsmActionResponse, Error> {
    self
      .shasta_client()
      .hsm_component_delete_one(auth_token, id)
      .await
      .map(Into::into)
      .map_err(Error::from)
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
        .split(',')
        .map(|regex_str| Regex::new(regex_str.trim()))
        .collect::<Result<Vec<Regex>, regex::Error>>()
        .map_err(|e| Error::Message(e.to_string()))?;

      // Get all HSM components (list of xnames + nids)
      // `Component100Component.components` is a `Vec` (with
      // `#[serde(default)]` for an absent `Components` array), so it is
      // already empty by default — no `unwrap_or_default()` needed.
      let hsm_component_vec = self
        .shasta_client()
        .hsm_component_get_all_nodes(shasta_token, Some("true"))
        .await
        .map_err(Error::from)?
        .components;

      let mut xname_vec: Vec<String> = vec![];

      // Get list of xnames the user is asking for
      for hsm_component in hsm_component_vec {
        let nid_long = format!(
          "nid{:06}",
          &hsm_component
            .nid
            .ok_or(crate::Error::ValidationFailed("No NID found"))?
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
                .as_ref()
                .ok_or(crate::Error::ValidationFailed("No XName found"))?
                .0
                .clone(),
            );
          }
        }
      }

      Ok(xname_vec)
    } else {
      log::debug!(
        "No regex found, getting xnames from list of NIDs or NIDs hostlist"
      );
      let nid_hostlist_expanded_vec = parse(user_input_nid).map_err(|e| {
        Error::Message(format!(
          "Could not parse list of nodes as a hostlist. Reason:\n{e}Exit"
        ))
      })?;

      log::debug!("hostlist: {user_input_nid}");
      log::debug!("hostlist expanded: {nid_hostlist_expanded_vec:?}");

      let mut nid_short_vec = Vec::new();

      for nid_long in nid_hostlist_expanded_vec {
        let nid_short_elem = nid_long
          .strip_prefix("nid")
          .ok_or_else(|| {
            Error::Message(format!(
              "Nid '{nid_long}' not valid, 'nid' prefix missing"
            ))
          })?
          .trim_start_matches('0');

        nid_short_vec.push(nid_short_elem.to_string());
      }

      let nid_short = nid_short_vec.join(",");

      log::debug!("short NID list: {nid_short}");

      let hsm_components = self
        .shasta_client()
        .hsm_component_get(
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
          Some("true"),
        )
        .await
        .map_err(Error::from)?;

      // Get list of xnames from HSM components. `components` is now a
      // `Vec` (with `#[serde(default)]` for an absent `Components`
      // array), and `id` is `Option<XName100>` — unwrap the newtype to
      // recover the inner `String`.
      let xname_vec: Vec<String> = hsm_components
        .components
        .iter()
        .filter_map(|component| component.id.as_ref().map(|x| x.0.clone()))
        .collect();

      log::debug!("xname list:\n{xname_vec:#?}");

      Ok(xname_vec)
    }
  }
}

impl ComponentEthernetInterfaceTrait for Csm {
  async fn get_all_component_ethernet_interfaces(
    &self,
    _auth_token: &str,
  ) -> Result<Vec<ComponentEthernetInterface>, Error> {
    Err(Error::Message(
      "Get all ethernet interfaces command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn get_component_ethernet_interface(
    &self,
    _auth_token: &str,
    _eth_interface_id: &str,
  ) -> Result<ComponentEthernetInterface, Error> {
    Err(Error::Message(
      "Get ethernet interfaces command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn update_component_ethernet_interface(
    &self,
    _auth_token: &str,
    _eth_interface_id: &str,
    _description: Option<&str>,
    _ip_address_mapping: (&str, &str),
  ) -> Result<Value, Error> {
    Err(Error::Message(
      "Update ethernet interface command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn delete_all_component_ethernet_interfaces(
    &self,
    _auth_token: &str,
  ) -> Result<Value, Error> {
    Err(Error::Message(
      "Delete all ethernet interface command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn delete_component_ethernet_interface(
    &self,
    _auth_token: &str,
    _eth_interface_id: &str,
  ) -> Result<Value, Error> {
    Err(Error::Message(
      "Delete ethernet interface command not implemented for this backend"
        .to_string(),
    ))
  }
}

impl RedfishEndpointTrait for Csm {
  async fn get_all_redfish_endpoints(
    &self,
    _auth_token: &str,
  ) -> Result<FrontEndRedfishEndpointArray, Error> {
    Err(Error::Message(
      "Get all redfish endpoint command not implemented for this backend"
        .to_string(),
    ))
  }
  async fn get_redfish_endpoints(
    &self,
    auth_token: &str,
    id: Option<&str>,
    fqdn: Option<&str>,
    r#type: Option<&str>,
    uuid: Option<&str>,
    macaddr: Option<&str>,
    ip_address: Option<&str>,
    last_status: Option<&str>,
  ) -> Result<FrontEndRedfishEndpointArray, Error> {
    self
      .shasta_client()
      .hsm_redfish_get(
        auth_token,
        id,
        fqdn,
        r#type,
        uuid,
        macaddr,
        ip_address,
        last_status,
      )
      .await
      .map(std::convert::Into::into)
      .map_err(Error::from)
  }

  async fn add_redfish_endpoint(
    &self,
    _auth_token: &str,
    _redfish_endpoint: &manta_backend_dispatcher::types::hsm::inventory::RedfishEndpointArray,
  ) -> Result<(), Error> {
    Err(Error::Message(
      "Add redfish endpoint command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn update_redfish_endpoint(
    &self,
    _auth_token: &str,
    _redfish_endpoint: &manta_backend_dispatcher::types::hsm::inventory::RedfishEndpoint,
  ) -> Result<(), Error> {
    Err(Error::Message(
      "Update redfish endpoint command not implemented for this backend"
        .to_string(),
    ))
  }

  async fn delete_redfish_endpoint(
    &self,
    _auth_token: &str,
    _id: &str,
  ) -> Result<Value, Error> {
    Err(Error::Message(
      "Delete redfish endpoint command not implemented for this backend"
        .to_string(),
    ))
  }
}
