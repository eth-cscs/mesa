use backend_dispatcher::types::cfs::cfs_configuration_response::{
    AdditionalInventory as FrontEndAdditionalInventory,
    CfsConfigurationResponse as FrontendCfsConfigurationResponse,
    CfsConfigurationVecResponse as FrontendCfsConfigurationVecResponse, Layer as FrontendLayer,
    Next as FrontendNext,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Layer {
    pub name: String,
    // #[serde(rename = "cloneUrl")]
    pub clone_url: String,
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub commit: Option<String>,
    pub playbook: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub branch: Option<String>,
}

impl From<FrontendLayer> for Layer {
    fn from(frontend_layer: FrontendLayer) -> Self {
        Self {
            name: frontend_layer.name,
            clone_url: frontend_layer.clone_url,
            source: frontend_layer.source,
            commit: frontend_layer.commit,
            playbook: frontend_layer.playbook,
            branch: frontend_layer.branch,
        }
    }
}

impl Into<FrontendLayer> for Layer {
    fn into(self) -> FrontendLayer {
        FrontendLayer {
            name: self.name,
            clone_url: self.clone_url,
            source: self.source,
            commit: self.commit,
            playbook: self.playbook,
            branch: self.branch,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AdditionalInventory {
    #[serde(rename = "cloneUrl")]
    pub clone_url: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub commit: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub branch: Option<String>,
}

impl From<FrontEndAdditionalInventory> for AdditionalInventory {
    fn from(value: FrontEndAdditionalInventory) -> Self {
        Self {
            clone_url: value.clone_url,
            commit: value.commit,
            name: value.name,
            branch: value.branch,
        }
    }
}

impl Into<FrontEndAdditionalInventory> for AdditionalInventory {
    fn into(self) -> FrontEndAdditionalInventory {
        FrontEndAdditionalInventory {
            clone_url: self.clone_url,
            commit: self.commit,
            name: self.name,
            branch: self.branch,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfsConfigurationResponse {
    pub name: String,
    // #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub layers: Vec<Layer>,
    #[serde(skip_serializing_if = "Option::is_none")] // Either commit or branch is passed
    pub additional_inventory: Option<AdditionalInventory>,
}

impl From<FrontendCfsConfigurationResponse> for CfsConfigurationResponse {
    fn from(value: FrontendCfsConfigurationResponse) -> Self {
        CfsConfigurationResponse {
            name: value.name,
            last_updated: value.last_updated,
            layers: value.layers.into_iter().map(Layer::from).collect(),
            additional_inventory: value.additional_inventory.map(AdditionalInventory::from),
        }
    }
}

impl Into<FrontendCfsConfigurationResponse> for CfsConfigurationResponse {
    fn into(self) -> FrontendCfsConfigurationResponse {
        FrontendCfsConfigurationResponse {
            name: self.name,
            last_updated: self.last_updated,
            layers: self.layers.into_iter().map(Into::into).collect(),
            additional_inventory: self.additional_inventory.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfsConfigurationVecResponse {
    pub configurations: Vec<CfsConfigurationResponse>,
    pub next: Option<Next>,
}

impl From<FrontendCfsConfigurationVecResponse> for CfsConfigurationVecResponse {
    fn from(value: FrontendCfsConfigurationVecResponse) -> Self {
        CfsConfigurationVecResponse {
            configurations: value
                .configurations
                .into_iter()
                .map(CfsConfigurationResponse::from)
                .collect(),
            next: value.next.map(Next::from),
        }
    }
}

impl Into<FrontendCfsConfigurationVecResponse> for CfsConfigurationVecResponse {
    fn into(self) -> FrontendCfsConfigurationVecResponse {
        FrontendCfsConfigurationVecResponse {
            configurations: self.configurations.into_iter().map(Into::into).collect(),
            next: self.next.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Next {
    limit: Option<u8>,
    after_id: Option<String>,
    in_use: Option<bool>,
}

impl From<FrontendNext> for Next {
    fn from(value: FrontendNext) -> Self {
        Next {
            limit: value.limit,
            after_id: value.after_id,
            in_use: value.in_use,
        }
    }
}

impl Into<FrontendNext> for Next {
    fn into(self) -> FrontendNext {
        FrontendNext {
            limit: self.limit,
            after_id: self.after_id,
            in_use: self.in_use,
        }
    }
}
