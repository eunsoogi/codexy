use std::{path::Path, process::Command};

use serde_json::{Map, Value};

const SOURCE_METHODS: [&str; 2] = ["read_thread", "rollout_jsonl"];

pub(crate) fn check_source(source: &Map<String, Value>) -> Result<(), String> {
    reject_unknown(
        source,
        &["provider", "method", "authenticated", "host_id"],
        "source",
    )?;
    if text(source, "provider", "source")? != "codex_app"
        || !SOURCE_METHODS.contains(&text(source, "method", "source")?)
        || source.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("pre-PR source must be an authenticated Codex app readback".into());
    }
    text(source, "host_id", "source")?;
    Ok(())
}

pub(crate) fn check_issue(issue: &Map<String, Value>) -> Result<u64, String> {
    reject_unknown(issue, &["repository", "number", "url"], "owning issue")?;
    let repository = text(issue, "repository", "owning issue")?;
    if repository.split('/').count() != 2 || repository.split('/').any(str::is_empty) {
        return Err("pre-PR owning issue repository is invalid".into());
    }
    let number = issue
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| "pre-PR owning issue number is invalid".to_owned())?;
    if text(issue, "url", "owning issue")?
        != format!("https://github.com/{repository}/issues/{number}")
    {
        return Err("pre-PR owning issue URL is not canonical".into());
    }
    Ok(number)
}

pub(crate) fn check_ancestor(
    repository_root: &Path,
    ancestor: &str,
    descendant: &str,
    label: &str,
) -> Result<(), String> {
    let object = format!("{ancestor}^{{commit}}");
    let exists = Command::new("git")
        .args(["cat-file", "-e", object.as_str()])
        .current_dir(repository_root)
        .status()
        .map_err(|_| format!("{label} commit lookup failed"))?;
    if !exists.success() {
        return Err(format!(
            "{label} references a commit unavailable in the repository"
        ));
    }
    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(repository_root)
        .status()
        .map_err(|_| format!("{label} ancestry check failed"))?;
    if !status.success() {
        return Err(format!(
            "{label} is not ordered through the current PR head"
        ));
    }
    Ok(())
}

pub(crate) fn object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a Map<String, Value>, String> {
    value
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{label} must be an object"))
}

pub(crate) fn text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} must contain non-empty {key}"))
}

pub(crate) fn number(object: &Map<String, Value>, key: &str, label: &str) -> Result<u64, String> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label} must contain numeric {key}"))
}

pub(crate) fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), String> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{label} contains an unknown field"));
    }
    Ok(())
}
