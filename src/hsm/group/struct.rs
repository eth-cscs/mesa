use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HsmGroup {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Member>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "exclusiveGroup"))]
    pub exclusive_group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Member {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct XnameId {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}
