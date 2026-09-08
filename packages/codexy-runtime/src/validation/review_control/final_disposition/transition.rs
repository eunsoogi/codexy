use std::path::Path;

use serde_json::{Map, Value};

pub(super) fn check(
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    previous_control: &Map<String, Value>,
    current_control: &Map<String, Value>,
    previous_count: u64,
    current_count: u64,
    previous_history: &[Value],
    current_history: &[Value],
) -> Result<bool, String> {
    if !current_control.contains_key("final_disposition") {
        return Ok(false);
    }
    if previous_count != 3
        || current_count != 3
        || previous_control.contains_key("final_disposition")
    {
        return Err(
            "review control final disposition is only valid after the third terminal verdict"
                .into(),
        );
    }
    if current_history != previous_history {
        return Err("final disposition must preserve the complete terminal review history".into());
    }
    super::repository::check(repository_root, previous, current, current_control)?;
    Ok(true)
}
