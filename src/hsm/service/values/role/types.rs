use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    #[serde(rename(serialize = "Role"))]
    pub role: Vec<String>,
}
