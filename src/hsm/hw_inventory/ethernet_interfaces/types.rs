use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IpAddressMapping {
    pub ip_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ComponentEthernetInterface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ip_addresses: Vec<IpAddressMapping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ComponentType {
    CDU,
    CabinetCDU,
    CabinetPDU,
    CabinetPDUOutlet,
    CabinetPDUPowerConnector,
    CabinetPDUController,
    r#Cabinet,
    Chassis,
    ChassisBMC,
    CMMRectifier,
    CMMFpga,
    CEC,
    ComputeModule,
    RouterModule,
    NodeBMC,
    NodeEnclosure,
    NodeEnclosurePowerSupply,
    HSNBoard,
    Node,
    Processor,
    Drive,
    StorageGroup,
    NodeNIC,
    Memory,
    NodeAccel,
    NodeAccelRiser,
    NodeFpga,
    HSNAsic,
    RouterFpga,
    RouterBMC,
    HSNLink,
    HSNConnector,
    INVALID,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EthernetInterface {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    mac_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_update: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<ComponentType>,
}
