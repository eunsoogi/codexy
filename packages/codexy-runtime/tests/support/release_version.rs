use serde_json::Value;

pub(super) fn current_plugin_release(
    plugin_root: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    manifest["version"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "plugin fixture version is missing".into())
}

pub(super) fn next_plugin_release(release: &str) -> Result<String, Box<dyn std::error::Error>> {
    let core_end = release
        .find(|character| matches!(character, '-' | '+'))
        .unwrap_or(release.len());
    let mut components = release[..core_end].split('.');
    let major = release_component(components.next(), "major")?;
    let minor = release_component(components.next(), "minor")?;
    let patch = release_component(components.next(), "patch")?;
    if components.next().is_some() {
        return Err("plugin fixture version has too many numeric components".into());
    }
    let next_patch = patch
        .checked_add(1)
        .ok_or("plugin fixture patch version cannot be incremented")?;
    Ok(format!("{major}.{minor}.{next_patch}"))
}

fn release_component(
    component: Option<&str>,
    name: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    component
        .ok_or_else(|| format!("plugin fixture version is missing {name} component"))?
        .parse()
        .map_err(|_| format!("plugin fixture version has invalid {name} component").into())
}
