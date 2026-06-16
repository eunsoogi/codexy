use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{load_json, manifest_path, require_string};

pub(super) fn load_manifest(plugin_root: &Path) -> Result<Value> {
    let path = manifest_path(plugin_root);
    let manifest = load_json(&path)?;
    if !manifest.is_object() {
        bail!("{} must contain a JSON object", display_relative(&path));
    }
    require_string(manifest.get("name"), "name", &path)?;
    require_string(manifest.get("version"), "version", &path)?;
    let interface = manifest
        .get("interface")
        .and_then(Value::as_object)
        .with_context(|| format!("{} interface must be an object", display_relative(&path)))?;
    require_string(interface.get("displayName"), "interface.displayName", &path)?;
    require_string(
        interface.get("shortDescription"),
        "interface.shortDescription",
        &path,
    )?;
    let default_prompt = interface
        .get("defaultPrompt")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .with_context(|| {
            format!(
                "{} interface.defaultPrompt must be a non-empty array",
                display_relative(&path)
            )
        })?;
    if !default_prompt
        .iter()
        .all(|item| item.as_str().is_some_and(|text| !text.trim().is_empty()))
    {
        bail!(
            "{} interface.defaultPrompt must contain only non-empty strings",
            display_relative(&path)
        );
    }
    Ok(manifest)
}

pub(super) fn mcp_config_path(plugin_root: &Path, manifest: &Value) -> Result<PathBuf> {
    let manifest_file = manifest_path(plugin_root);
    let configured = manifest
        .get("mcpServers")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "{} mcpServers must be a path string",
                display_relative(&manifest_file)
            )
        })?;
    let configured_path = Path::new(configured);
    if configured_path.is_absolute() {
        bail!(
            "{} mcpServers must be plugin-relative",
            display_relative(&manifest_file)
        );
    }
    let resolved = plugin_root
        .join(configured_path)
        .canonicalize()
        .or_else(|_| {
            plugin_root.join(configured_path).parent().map_or_else(
                || Ok(plugin_root.join(configured_path)),
                |parent| {
                    parent
                        .canonicalize()
                        .map(|root| root.join(configured_path.file_name().unwrap_or_default()))
                },
            )
        })?;
    let plugin_root_resolved = plugin_root.canonicalize()?;
    if !resolved.starts_with(&plugin_root_resolved) {
        bail!(
            "{} mcpServers must stay inside the plugin root",
            display_relative(&manifest_file)
        );
    }
    Ok(resolved)
}

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match load_manifest(plugin_root).and_then(|manifest| {
        let mcp_path = mcp_config_path(plugin_root, &manifest)?;
        if !mcp_path.exists() {
            bail!(
                "manifest mcpServers target does not exist: {}",
                display_relative(&mcp_path)
            );
        }
        Ok(())
    }) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}
