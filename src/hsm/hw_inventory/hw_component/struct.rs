use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

#[derive(
    Debug, EnumIter, EnumString, IntoStaticStr, AsRefStr, Display, Serialize, Deserialize, Clone,
)]
pub enum ArtifactType {
    Memory,
    Processor,
    NodeAccel,
    NodeHsnNic,
    Drive,
    CabinetPDU,
    CabinetPDUPowerConnector,
    CMMRectifier,
    NodeAccelRiser,
    NodeEnclosurePowerSupplie,
    NodeBMC,
    RouterBMC,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeSummary {
    pub xname: String,
    pub r#type: String,
    pub processors: Vec<ArtifactSummary>,
    pub memory: Vec<ArtifactSummary>,
    pub node_accels: Vec<ArtifactSummary>,
    pub node_hsn_nics: Vec<ArtifactSummary>,
}

impl NodeSummary {
    pub fn from_csm_value(hw_artifact_value: Value) -> Self {
        let processors = hw_artifact_value["Processors"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|processor_value| ArtifactSummary::from_processor_value(processor_value.clone()))
            .collect();

        let memory = hw_artifact_value["Memory"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|memory_value| ArtifactSummary::from_memory_value(memory_value.clone()))
            .collect();

        let node_accels = hw_artifact_value["NodeAccels"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|nodeaccel_value| ArtifactSummary::from_nodeaccel_value(nodeaccel_value.clone()))
            .collect();

        let node_hsn_nics = hw_artifact_value["NodeHsnNics"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|nodehsnnic_value| {
                ArtifactSummary::from_nodehsnnics_value(nodehsnnic_value.clone())
            })
            .collect();

        Self {
            xname: hw_artifact_value["ID"].as_str().unwrap().to_string(),
            r#type: hw_artifact_value["Type"].as_str().unwrap().to_string(),
            processors,
            memory,
            node_accels,
            node_hsn_nics,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactSummary {
    pub xname: String,
    pub r#type: ArtifactType,
    pub info: Option<String>,
}

impl ArtifactSummary {
    fn from_processor_value(processor_value: Value) -> Self {
        Self {
            xname: processor_value["ID"].as_str().unwrap().to_string(),
            r#type: ArtifactType::from_str(processor_value["Type"].as_str().unwrap()).unwrap(),
            info: processor_value
                .pointer("/PopulatedFRU/ProcessorFRUInfo/Model")
                .map(|model| model.as_str().unwrap().to_string()),
        }
    }

    fn from_memory_value(memory_value: Value) -> Self {
        Self {
            xname: memory_value["ID"].as_str().unwrap().to_string(),
            r#type: ArtifactType::from_str(memory_value["Type"].as_str().unwrap()).unwrap(),
            info: memory_value
                .pointer("/PopulatedFRU/MemoryFRUInfo/CapacityMiB")
                .map(|capacity_mib| capacity_mib.as_number().unwrap().to_string() + " MiB"),
        }
    }

    fn from_nodehsnnics_value(nodehsnnic_value: Value) -> Self {
        Self {
            xname: nodehsnnic_value["ID"].as_str().unwrap().to_string(),
            r#type: ArtifactType::from_str(nodehsnnic_value["Type"].as_str().unwrap()).unwrap(),
            info: nodehsnnic_value
                .pointer("/NodeHsnNicLocationInfo/Description")
                .map(|description| description.as_str().unwrap().to_string()),
        }
    }

    fn from_nodeaccel_value(nodeaccel_value: Value) -> Self {
        Self {
            xname: nodeaccel_value["ID"].as_str().unwrap().to_string(),
            r#type: ArtifactType::from_str(nodeaccel_value["Type"].as_str().unwrap()).unwrap(),
            info: nodeaccel_value
                .pointer("/PopulatedFRU/NodeAccelFRUInfo/Model")
                .map(|model| model.as_str().unwrap().to_string()),
        }
    }
}
