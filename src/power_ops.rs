use serde_json::Value;

use crate::error::Error;

pub trait Power {
    // FIXME: Create a new type PowerStatus and return Result<PowerStatus, Error>
    fn power_off(base_url: &str, auth_token: &str, root_cert: &[u8]) -> Result<Value, Error>;
}
