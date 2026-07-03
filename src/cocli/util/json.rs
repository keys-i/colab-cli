use serde::Serialize;

use crate::cocli::error::Result;

pub fn to_pretty<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}
