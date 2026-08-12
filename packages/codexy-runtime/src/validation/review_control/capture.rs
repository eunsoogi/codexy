use anyhow::{Result, bail};
use serde_json::Value;

use super::terminal;

pub(super) fn build_pr_state(
    plugin_root: &std::path::Path,
    base_text: &str,
    control_text: &str,
) -> Result<Value> {
    let mut state: Value = serde_json::from_str(base_text)?;
    let control: Value = serde_json::from_str(control_text)?;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("base PR state must be an object"))?;
    if object.contains_key("reviewControl") {
        bail!("base PR state must not already contain review control fields");
    }
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    object.insert("reviewControl".into(), control);
    if let Some(error) = terminal::check_handoff(plugin_root, &state)
        .into_iter()
        .next()
    {
        bail!(error);
    }
    Ok(state)
}
