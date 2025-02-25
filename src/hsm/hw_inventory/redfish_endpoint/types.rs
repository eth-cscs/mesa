use backend_dispatcher::types::hsm::inventory::{
    DiscoveryInfo as FrontEndDiscoveryInfo, RedfishEndpoint as FrontEndRedfishEndpoint,
    RedfishEndpointArray as FrontEndRedfishEndpointArray,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoveryInfo {
    #[serde(rename(serialize = "LastAttempt"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempt: Option<String>,
    #[serde(rename(serialize = "LastStatus"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(rename(serialize = "RedfishVersion"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    redfish_version: Option<String>,
}

impl From<FrontEndDiscoveryInfo> for DiscoveryInfo {
    fn from(info: FrontEndDiscoveryInfo) -> Self {
        DiscoveryInfo {
            last_attempt: info.last_attempt,
            last_status: info.last_status,
            redfish_version: info.redfish_version,
        }
    }
}

impl Into<FrontEndDiscoveryInfo> for DiscoveryInfo {
    fn into(self) -> FrontEndDiscoveryInfo {
        FrontEndDiscoveryInfo {
            last_attempt: self.last_attempt,
            last_status: self.last_status,
            redfish_version: self.redfish_version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedfishEndpoint {
    #[serde(rename(serialize = "ID"))]
    pub id: String,
    #[serde(rename(serialize = "Type"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename(serialize = "Name"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename(serialize = "Hostname"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(rename(serialize = "Domain"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(rename(serialize = "FQDN"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fqdn: Option<String>,
    #[serde(rename(serialize = "Enabled"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename(serialize = "UUID"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(rename(serialize = "User"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(rename(serialize = "Password"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename(serialize = "UseSSDP"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_ssdp: Option<bool>,
    #[serde(rename(serialize = "MacRequired"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_required: Option<bool>,
    #[serde(rename(serialize = "MACAddr"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_addr: Option<String>,
    #[serde(rename(serialize = "IPAddress"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(rename(serialize = "RediscoveryOnUpdate"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rediscover_on_update: Option<bool>,
    #[serde(rename(serialize = "TemplateID"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(rename(serialize = "DiscoveryInfo"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_info: Option<DiscoveryInfo>,
}

impl From<FrontEndRedfishEndpoint> for RedfishEndpoint {
    fn from(endpoint: FrontEndRedfishEndpoint) -> Self {
        RedfishEndpoint {
            id: endpoint.id,
            r#type: endpoint.r#type,
            name: endpoint.name,
            hostname: endpoint.hostname,
            domain: endpoint.domain,
            fqdn: endpoint.fqdn,
            enabled: endpoint.enabled,
            uuid: endpoint.uuid,
            user: endpoint.user,
            password: endpoint.password,
            use_ssdp: endpoint.use_ssdp,
            mac_required: endpoint.mac_required,
            mac_addr: endpoint.mac_addr,
            ip_address: endpoint.ip_address,
            rediscover_on_update: endpoint.rediscover_on_update,
            template_id: endpoint.template_id,
            discovery_info: endpoint.discovery_info.map(|info| info.into()),
        }
    }
}

impl Into<FrontEndRedfishEndpoint> for RedfishEndpoint {
    fn into(self) -> FrontEndRedfishEndpoint {
        FrontEndRedfishEndpoint {
            id: self.id,
            r#type: self.r#type,
            name: self.name,
            hostname: self.hostname,
            domain: self.domain,
            fqdn: self.fqdn,
            enabled: self.enabled,
            uuid: self.uuid,
            user: self.user,
            password: self.password,
            use_ssdp: self.use_ssdp,
            mac_required: self.mac_required,
            mac_addr: self.mac_addr,
            ip_address: self.ip_address,
            rediscover_on_update: self.rediscover_on_update,
            template_id: self.template_id,
            discovery_info: self.discovery_info.map(|info| info.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedfishEndpointArray {
    #[serde(rename(serialize = "RedfishEndpoints"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redfish_endpoints: Option<Vec<RedfishEndpoint>>,
}

impl From<FrontEndRedfishEndpointArray> for RedfishEndpointArray {
    fn from(array: FrontEndRedfishEndpointArray) -> Self {
        RedfishEndpointArray {
            redfish_endpoints: array
                .redfish_endpoints
                .map(|endpoints| endpoints.into_iter().map(RedfishEndpoint::from).collect()),
        }
    }
}

impl Into<FrontEndRedfishEndpointArray> for RedfishEndpointArray {
    fn into(self) -> FrontEndRedfishEndpointArray {
        FrontEndRedfishEndpointArray {
            redfish_endpoints: self.redfish_endpoints.map(|endpoints| {
                endpoints
                    .into_iter()
                    .map(|endpoint| endpoint.into())
                    .collect()
            }),
        }
    }
}
