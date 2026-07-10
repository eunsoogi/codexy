pub(super) fn set_plugin_release(
    plugin_root: &std::path::Path,
    current: &str,
    next: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let manifest = std::fs::read_to_string(&manifest_path)?;
    let current_field = format!("\"version\": \"{current}\"");
    if !manifest.contains(&current_field) {
        return Err(format!("plugin fixture version {current} not found").into());
    }
    std::fs::write(
        manifest_path,
        manifest.replacen(&current_field, &format!("\"version\": \"{next}\""), 1),
    )?;
    Ok(())
}
