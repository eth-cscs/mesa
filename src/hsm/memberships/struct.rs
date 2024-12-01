use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Membership {
    pub id: String,
    #[serde(rename = "partitionName")]
    pub partition_name: String,
    #[serde(rename = "groupLabels")]
    pub group_labels: Vec<String>,
}
