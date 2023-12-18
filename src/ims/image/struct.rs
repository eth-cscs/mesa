use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Link {
    pub path: String,
    pub etag: Option<String>,
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Image {
    pub id: Option<String>,
    pub created: Option<String>,
    pub name: String,
    pub link: Option<Link>,
}

