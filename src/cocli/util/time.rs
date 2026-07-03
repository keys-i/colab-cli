use crate::cocli::error::{ColabError, Result};

pub fn parse_days(s: &str) -> Result<u64> {
    let Some(days) = s.strip_suffix('d') else {
        return Err(ColabError::config(
            "duration must use whole days, for example 7d",
        ));
    };
    days.parse::<u64>()
        .map_err(|_| ColabError::config(format!("invalid day duration: {s}")))
}
