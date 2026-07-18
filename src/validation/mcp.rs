use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json};

const REQUIRED_MCP_NAMES: &[&str] = &["lsp", "codegraph"];
const DISALLOWED_NAMES: &[&str] = &["context7"];
const DISALLOWED_FRAGMENTS: &[&str] = &["openai", "context7"];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match check_inner(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

fn check_inner(plugin_root: &Path) -> Result<()> {
    let manifest = super::manifest::load_manifest(plugin_root)?;
    let path = super::manifest::mcp_config_path(plugin_root, &manifest)?;
    let data = load_json(&path)?;
    if data.get("mcpServers").is_some() {
        bail!(
            "{} must use a direct server map or 'mcp_servers', not 'mcpServers'",
            display_relative(&path)
        );
    }
    let servers = data
        .get("mcp_servers")
        .unwrap_or(&data)
        .as_object()
        .with_context(|| {
            format!(
                "{} MCP server map must be an object",
                display_relative(&path)
            )
        })?;
    if data.get("mcp_servers").is_some() && data.as_object().is_some_and(|items| items.len() != 1) {
        bail!(
            "{} must not mix 'mcp_servers' with direct MCP server entries",
            display_relative(&path)
        );
    }
    let missing = REQUIRED_MCP_NAMES
        .iter()
        .filter(|name| !servers.contains_key(**name))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "{} missing required MCP servers: {}",
            display_relative(&path),
            missing.join(", ")
        );
    }
    for (name, entry) in servers {
        check_entry(&path, plugin_root, name, entry)?;
    }
    Ok(())
}

fn check_entry(path: &Path, plugin_root: &Path, name: &str, entry: &Value) -> Result<()> {
    if name.is_empty() {
        bail!(
            "{} MCP names must be non-empty strings",
            display_relative(path)
        );
    }
    let lowered_name = name.to_ascii_lowercase();
    if DISALLOWED_NAMES.contains(&name)
        || DISALLOWED_FRAGMENTS
            .iter()
            .any(|fragment| lowered_name.contains(fragment))
    {
        bail!(
            "{} disallowed MCP server present: {name}",
            display_relative(path)
        );
    }
    let object = entry
        .as_object()
        .with_context(|| format!("{} {name} must be an object", display_relative(path)))?;
    if object.contains_key("commandWindows") || object.contains_key("command_windows") {
        bail!(
            "{} {name} must not use hook-only commandWindows in MCP config",
            display_relative(path)
        );
    }
    if let Some(fragment) = disallowed_value_fragments(entry).into_iter().next() {
        bail!(
            "{} disallowed MCP value fragment {fragment:?} present for {name}",
            display_relative(path)
        );
    }
    let url = object.get("url");
    let command = object.get("command");
    if url.is_none() && command.is_none() {
        bail!(
            "{} {name} must define either url or command",
            display_relative(path)
        );
    }
    if let Some(url) = url.and_then(Value::as_str) {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            bail!(
                "{} {name}.url must be an HTTP(S) string",
                display_relative(path)
            );
        }
    } else if url.is_some() {
        bail!(
            "{} {name}.url must be an HTTP(S) string",
            display_relative(path)
        );
    }
    if let Some(command_value) = command {
        let command_items = command_items(path, name, command_value, object.get("args"))?;
        super::mcp_required::check(path, name, object, &command_items)?;
        super::mcp_runtime::check_no_script_runtime(path, name, &command_items)?;
        check_plugin_relative_entrypoint(path, plugin_root, name, &command_items)?;
    }
    if let Some(cwd) = object.get("cwd") {
        if !cwd.is_string() {
            bail!("{} {name}.cwd must be a string", display_relative(path));
        }
    }
    Ok(())
}

fn command_items(
    path: &Path,
    name: &str,
    command_value: &Value,
    args_value: Option<&Value>,
) -> Result<Vec<String>> {
    if let Some(command) = command_value.as_str() {
        if command.trim().is_empty() {
            bail!(
                "{} {name}.command must be a non-empty string",
                display_relative(path)
            );
        }
        let args = match args_value {
            Some(value) => json_array_strings(Some(value)).with_context(|| {
                format!(
                    "{} {name}.args must be an array of strings",
                    display_relative(path)
                )
            })?,
            None => Vec::new(),
        };
        if args.iter().any(|item| item.trim().is_empty()) {
            bail!(
                "{} {name}.args must contain only non-empty strings",
                display_relative(path)
            );
        }
        let mut items = Vec::with_capacity(args.len() + 1);
        items.push(command.to_owned());
        items.extend(args);
        return Ok(items);
    }
    let command_items = json_array_strings(Some(command_value))
        .filter(|items| !items.is_empty() && items.iter().all(|item| !item.trim().is_empty()));
    let Some(command_items) = command_items else {
        bail!(
            "{} {name}.command must be a non-empty string or argv array",
            display_relative(path)
        );
    };
    if args_value.is_some() {
        bail!(
            "{} {name}.args is only allowed with string command",
            display_relative(path)
        );
    }
    Ok(command_items)
}

fn check_plugin_relative_entrypoint(
    path: &Path,
    plugin_root: &Path,
    name: &str,
    command: &[String],
) -> Result<()> {
    let canonical_root = plugin_root.canonicalize().with_context(|| {
        format!(
            "{} plugin root cannot be canonicalized",
            display_relative(plugin_root)
        )
    })?;
    for entrypoint in command
        .iter()
        .filter(|item| item.starts_with("./") || item.starts_with("../"))
    {
        let resolved = plugin_root.join(Path::new(entrypoint));
        if !resolved.exists() {
            bail!(
                "{} {name}.command entrypoint does not exist: {entrypoint}",
                display_relative(path)
            );
        }
        let canonical_resolved = resolved.canonicalize().with_context(|| {
            format!(
                "{} {name}.command entrypoint cannot be canonicalized: {entrypoint}",
                display_relative(path)
            )
        })?;
        if !canonical_resolved.starts_with(&canonical_root) {
            bail!(
                "{} {name}.command entrypoint must stay inside the plugin root",
                display_relative(path)
            );
        }
    }
    Ok(())
}

fn disallowed_value_fragments(value: &Value) -> Vec<&'static str> {
    let mut matches = Vec::new();
    collect_fragments(value, &mut matches);
    matches.sort_unstable();
    matches.dedup();
    matches
}

fn collect_fragments(value: &Value, matches: &mut Vec<&str>) {
    match value {
        Value::String(text) => {
            let lowered = text.to_ascii_lowercase();
            matches.extend(
                DISALLOWED_FRAGMENTS
                    .iter()
                    .copied()
                    .filter(|fragment| lowered.contains(fragment)),
            );
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_fragments(item, matches)),
        Value::Object(items) => items
            .values()
            .for_each(|item| collect_fragments(item, matches)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
