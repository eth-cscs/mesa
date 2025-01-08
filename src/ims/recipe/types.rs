use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Link {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RecipeGetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
    pub recipe_type: String,
    pub linux_distribution: String,
    pub name: String,
}
