use serde_json::{Map, Value};

use super::super::{policy, snapshot, state};
use super::{count, history, required_text};

pub(super) fn check(
    plugin_root: &std::path::Path,
    control: &Map<String, Value>,
) -> Result<(), String> {
    if control.get("schema").and_then(Value::as_str) != Some(state::CONTROL_SCHEMA)
        || control.contains_key("reviewed_head")
        || control.contains_key("terminal_result")
        || control.contains_key("post_cap_re_review")
        || control.contains_key("reviewer_migration")
        || count(control, "full_review_count")? != 0
        || count(control, "delta_review_count")? != 0
        || count(control, "terminal_review_count")? != 0
        || !history(control, "genesis")?.is_empty()
    {
        return Err("review control transition previous state is not a clean genesis".into());
    }
    if !control
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Err("review control transition genesis must have no findings".into());
    }
    let profile_name = required_text(control, "profile", "genesis")?;
    let profiles =
        policy::load(plugin_root).map_err(|_| "review profile policy is unavailable".to_owned())?;
    let profile = profiles
        .get(profile_name)
        .ok_or_else(|| "review control transition genesis selects an unknown profile".to_owned())?;
    let reviewer = profile
        .reviewer
        .as_ref()
        .ok_or_else(|| "review control transition genesis must select a reviewer".to_owned())?;
    let expected = serde_json::to_value(reviewer)
        .map_err(|_| "review control transition reviewer is not serializable".to_owned())?;
    if control.get("reviewer") != Some(&expected)
        || count(control, "terminal_review_limit")? != u64::from(profile.terminal_review_limit)
    {
        return Err("review control transition genesis does not bind policy".into());
    }
    Ok(())
}

pub(super) fn check_snapshot(value: &Value, control: &Map<String, Value>) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "previous PR snapshot must be an object".to_owned())?;
    let issue = snapshot::owning_issue_number(value, "previous")?;
    if issue != count(control, "issue_number")? {
        return Err("genesis PR snapshot issue identity disagrees with review control".into());
    }
    if let Some(profile) = object.get("reviewProfile").and_then(Value::as_str)
        && control.get("profile").and_then(Value::as_str) != Some(profile)
    {
        return Err("genesis PR snapshot profile disagrees with review control".into());
    }
    Ok(())
}
