use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Location {
    pub xname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "deputyKey")]
    pub deputy_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Operation {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "soft-off")]
    SoftOff,
    #[serde(rename = "soft-restart")]
    SoftRestart,
    #[serde(rename = "hard-restart")]
    HardRestart,
    #[serde(rename = "init")]
    Init,
    #[serde(rename = "force-off")]
    ForceOff,
}

impl Operation {
    /* pub fn to_string(&self) -> String {
        match self {
            Operation::On => "on".to_string(),
            Operation::Off => "off".to_string(),
            Operation::SoftOff => "soft-off".to_string(),
            Operation::SoftRestart => "soft-restart".to_string(),
            Operation::HardRestart => "hard-restart".to_string(),
            Operation::Init => "init".to_string(),
            Operation::ForceOff => "force-off".to_string(),
        }
    } */

    pub fn from_str(operation: &str) -> Result<Operation, Error> {
        match operation {
            "on" => Ok(Operation::On),
            "off" => Ok(Operation::Off),
            "soft-off" => Ok(Operation::SoftOff),
            "soft-restart" => Ok(Operation::SoftRestart),
            "hard-restart" => Ok(Operation::HardRestart),
            "init" => Ok(Operation::Init),
            "force-off" => Ok(Operation::ForceOff),
            _ => Err(Error::Message("Operation not valid".to_string())),
        }
    }
}

/* impl FromStr for Operation {
    type Err = Error;

    fn from_str(operation: &str) -> Result<Operation, Error> {
        Self::from_str(operation)
    }
} */

#[derive(Debug, Serialize, Deserialize)]
pub struct Transition {
    pub operation: Operation,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskDeadlineMinutes")]
    pub task_deadline_minutes: Option<usize>,
    pub location: Vec<Location>,
}
