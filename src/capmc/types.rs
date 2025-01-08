use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PowerStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    xnames: Vec<String>,
    force: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    recursive: Option<bool>,
}

impl PowerStatus {
    pub fn new(
        reason: Option<String>,
        xnames: Vec<String>,
        force: bool,
        recursive: Option<bool>,
    ) -> Self {
        Self {
            reason,
            xnames,
            force,
            recursive,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NodeStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xnames: Option<Vec<String>>,
}

impl NodeStatus {
    pub fn new(
        filter: Option<String>,
        xnames: Option<Vec<String>>,
        source: Option<String>,
    ) -> Self {
        Self {
            filter,
            source,
            xnames,
        }
    }
}
