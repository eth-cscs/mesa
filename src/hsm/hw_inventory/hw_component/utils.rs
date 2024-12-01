use std::collections::HashMap;

use serde_json::Value;

use super::r#struct::NodeSummary;

pub fn get_list_processor_model_from_hw_inventory_value(
    hw_inventory: &Value,
) -> Option<Vec<String>> {
    hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["Processors"]
        .as_array()
        .map(|processor_list: &Vec<Value>| {
            processor_list
                .iter()
                .map(|processor| {
                    processor
                        .pointer("/PopulatedFRU/ProcessorFRUInfo/Model")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        })
}

pub fn get_list_accelerator_model_from_hw_inventory_value(
    hw_inventory: &Value,
) -> Option<Vec<String>> {
    hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["NodeAccels"]
        .as_array()
        .map(|accelerator_list| {
            accelerator_list
                .iter()
                .map(|accelerator| {
                    accelerator
                        .pointer("/PopulatedFRU/NodeAccelFRUInfo/Model")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        })
}

pub fn get_list_hsn_nics_model_from_hw_inventory_value(
    hw_inventory: &Value,
) -> Option<Vec<String>> {
    hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["NodeHsnNics"]
        .as_array()
        .map(|hsn_nic_list| {
            hsn_nic_list
                .iter()
                .map(|hsn_nic| {
                    hsn_nic
                        .pointer("/NodeHsnNicLocationInfo/Description")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        })
}

pub fn get_list_memory_capacity_from_hw_inventory_value(hw_inventory: &Value) -> Option<Vec<u64>> {
    hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["Memory"]
        .as_array()
        .map(|memory_list| {
            memory_list
                .iter()
                .map(|memory| {
                    memory
                        .pointer("/PopulatedFRU/MemoryFRUInfo/CapacityMiB")
                        .unwrap_or(&serde_json::json!(0))
                        .as_u64()
                        .unwrap()
                })
                .collect::<Vec<u64>>()
        })
}

pub fn calculate_hsm_hw_component_summary(
    node_summary_vec: &Vec<NodeSummary>,
) -> HashMap<String, usize> {
    let mut node_hw_component_summary: HashMap<String, usize> = HashMap::new();

    for node_summary in node_summary_vec {
        for artifact_summary in &node_summary.processors {
            node_hw_component_summary
                .entry(artifact_summary.info.as_ref().unwrap().to_string())
                .and_modify(|summary_quantity| *summary_quantity += 1)
                .or_insert(1);
        }
        for artifact_summary in &node_summary.node_accels {
            node_hw_component_summary
                .entry(artifact_summary.info.as_ref().unwrap().to_string())
                .and_modify(|summary_quantity| *summary_quantity += 1)
                .or_insert(1);
        }
        for artifact_summary in &node_summary.memory {
            let memory_capacity = artifact_summary
                .info
                .as_ref()
                .unwrap_or(&"ERROR NA".to_string())
                .split(' ')
                .collect::<Vec<_>>()
                .first()
                .unwrap()
                .parse::<usize>()
                .unwrap_or(0);
            node_hw_component_summary
                .entry(artifact_summary.r#type.to_string() + " (GiB)")
                .and_modify(|summary_quantity| {
                    *summary_quantity += memory_capacity / 1024;
                })
                .or_insert(memory_capacity / 1024);
        }
        for artifact_summary in &node_summary.node_hsn_nics {
            node_hw_component_summary
                .entry(artifact_summary.info.as_ref().unwrap().to_string())
                .and_modify(|summary_quantity| *summary_quantity += 1)
                .or_insert(1);
        }
    }

    node_hw_component_summary
}
