use crate::error::Error;

use super::types::{ComponentEthernetInterface, IpAddressMapping};

// Get list of network interfaces
// ref --> https://csm12-apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doCompEthInterfacesGetV2/
pub async fn get(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    mac_address: &str,
    ip_address: &str,
    network: &str,
    component_id: &str, // Node's xname
    r#type: &str,
    olther_than: &str,
    newer_than: &str,
) -> Result<reqwest::Response, Error> {
    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = shasta_base_url.to_owned() + "/smd/hsm/v2/Inventory/EthernetInterfaces";

    client
        .get(api_url)
        .query(&[
            ("MACAddress", mac_address),
            ("IPAddress", ip_address),
            ("Network", network),
            ("ComponentID", component_id),
            ("Type", r#type),
            ("OlderThan", olther_than),
            ("NewerThan", newer_than),
        ])
        .bearer_auth(shasta_token)
        .send()
        .await?
        .error_for_status()
        .map_err(Error::NetError)
}

pub async fn patch(
    shasta_token: &str,
    shasta_base_url: &str,
    shasta_root_cert: &[u8],
    eth_interface_id: &str,
    description: Option<&str>,
    component_id: &str,
    ip_address_mapping: (&str, &str), // [(<ip address>, <network>), ...], examle
                                      // [("192.168.1.10", "HMN"), ...]
) -> Result<reqwest::Response, Error> {
    let ip_address = ip_address_mapping.0;
    let network = ip_address_mapping.1;
    let cei = ComponentEthernetInterface {
        description: description.map(|value| value.to_string()),
        ip_addresses: vec![IpAddressMapping {
            ip_address: ip_address.to_string(),
            network: Some(network.to_string()),
        }],
        component_id: Some(component_id.to_string()),
    };

    let client_builder = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

    // Build client
    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
        // socks5 proxy
        log::debug!("SOCKS5 enabled");
        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

        // rest client to authenticate
        client_builder.proxy(socks5proxy).build()?
    } else {
        client_builder.build()?
    };

    let api_url: String = format!(
        "{}/smd/hsm/v2/Inventory/EthernetInterfaces/{}",
        shasta_base_url, eth_interface_id
    );

    client
        .patch(api_url)
        .query(&[("ethInterfaceID", ip_address), ("ipAddress", ip_address)])
        .bearer_auth(shasta_token)
        .json(&cei)
        .send()
        .await
        .map_err(Error::NetError)?
        .error_for_status()
        .map_err(Error::NetError)
}
