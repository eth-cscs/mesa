use std::io;

use serde_json::Value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("ERROR - MESA: {0}")]
    Message(String),
    #[error("ERROR - IO: {0}")]
    IoError(#[from] io::Error),
    #[error("ERROR - Serde: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("ERROR - Net: {0}")]
    NetError(#[from] reqwest::Error),
    #[error("ERROR - http request:\nresponse: {response}\npayload: {payload}")]
    RequestError {
        response: reqwest::Error,
        payload: String, // NOTE: CSM/OCHAMI Apis either returns plain text or a json therefore, we
                         // will just return a String
    },
    #[error("ERROR - CSM: {0}")]
    CsmError(Value),
    #[error("ERROR - Console: {0}")]
    ConsoleError(String),
    #[error("ERROR - K8s: {0}")]
    K8sError(String),
    #[error("ERROR - Image '{0}' not found")]
    ImageNotFound(String),
    #[error("ERROR - Group '{0}' not found")]
    GroupNotFound(String),
}
