use std::io;

use serde_json::Value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("ERROR - MESA: {0}")]
    Message(String),
    #[error("ERROR - IO: {0}")]
    IoError(#[from] io::Error),
    #[error("ERROR - Net: {0}")]
    NetError(#[from] reqwest::Error),
    #[error("ERROR - CSM: {0}")]
    CsmError(Value),
}
