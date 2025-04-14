use serde::{Deserialize, Serialize};

use backend_dispatcher::types::bos::session::{
    BosSession as FrontEndBosSession, Operation as FrontEndOperation, Status as FrontEndStatus,
    StatusLabel as FrontEndStatusLabel,
};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct BosSession {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<Operation>,
    pub template_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

impl From<FrontEndBosSession> for BosSession {
    fn from(frontend_bos_session: FrontEndBosSession) -> Self {
        Self {
            name: frontend_bos_session.name,
            tenant: frontend_bos_session.tenant,
            operation: frontend_bos_session
                .operation
                .map(|operation| operation.into()),
            template_name: frontend_bos_session.template_name,
            limit: frontend_bos_session.limit,
            stage: frontend_bos_session.stage,
            components: frontend_bos_session.components,
            include_disabled: frontend_bos_session.include_disabled,
            status: frontend_bos_session.status.map(|status| status.into()),
        }
    }
}

impl Into<FrontEndBosSession> for BosSession {
    fn into(self) -> FrontEndBosSession {
        FrontEndBosSession {
            name: self.name,
            tenant: self.tenant,
            operation: self.operation.map(|operation| operation.into()),
            template_name: self.template_name,
            limit: self.limit,
            stage: self.stage,
            components: self.components,
            include_disabled: self.include_disabled,
            status: self.status.map(|status| status.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Operation {
    #[serde(rename = "boot")]
    Boot,
    #[serde(rename = "reboot")]
    Reboot,
    #[serde(rename = "shutdown")]
    Shutdown,
}

impl From<FrontEndOperation> for Operation {
    fn from(frontend_operation: FrontEndOperation) -> Self {
        match frontend_operation {
            FrontEndOperation::Boot => Self::Boot,
            FrontEndOperation::Reboot => Self::Reboot,
            FrontEndOperation::Shutdown => Self::Shutdown,
        }
    }
}

impl Into<FrontEndOperation> for Operation {
    fn into(self) -> FrontEndOperation {
        match self {
            Operation::Boot => FrontEndOperation::Boot,
            Operation::Reboot => FrontEndOperation::Reboot,
            Operation::Shutdown => FrontEndOperation::Shutdown,
        }
    }
}

impl Operation {
    pub fn to_string(&self) -> String {
        match self {
            Operation::Boot => "boot".to_string(),
            Operation::Reboot => "reboot".to_string(),
            Operation::Shutdown => "shutdown".to_string(),
        }
    }

    pub fn from_str(operation: &str) -> Result<Operation, Error> {
        match operation {
            "boot" => Ok(Operation::Boot),
            "reboot" => Ok(Operation::Reboot),
            "shutdown" => Ok(Operation::Shutdown),
            _ => Err(Error::Message("Operation not valid".to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub status: StatusLabel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<FrontEndStatus> for Status {
    fn from(frontend_status: FrontEndStatus) -> Self {
        Self {
            start_time: frontend_status.start_time,
            end_time: frontend_status.end_time,
            status: frontend_status.status.into(),
            error: frontend_status.error,
        }
    }
}

impl Into<FrontEndStatus> for Status {
    fn into(self) -> FrontEndStatus {
        FrontEndStatus {
            start_time: self.start_time,
            end_time: self.end_time,
            status: self.status.into(),
            error: self.error,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StatusLabel {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "complete")]
    Complete,
}

impl From<FrontEndStatusLabel> for StatusLabel {
    fn from(frontend_status_label: FrontEndStatusLabel) -> Self {
        match frontend_status_label {
            FrontEndStatusLabel::Pending => Self::Pending,
            FrontEndStatusLabel::Running => Self::Running,
            FrontEndStatusLabel::Complete => Self::Complete,
        }
    }
}

impl Into<FrontEndStatusLabel> for StatusLabel {
    fn into(self) -> FrontEndStatusLabel {
        match self {
            StatusLabel::Pending => FrontEndStatusLabel::Pending,
            StatusLabel::Running => FrontEndStatusLabel::Running,
            StatusLabel::Complete => FrontEndStatusLabel::Complete,
        }
    }
}
