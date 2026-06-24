use std::{collections::BTreeMap, path::Path};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json, load_toml, toml_array_strings};

const REQUIRED_LSP_EXTENSIONS: &[&str] = &[".py", ".pyi", ".yaml", ".yml", ".json", ".toml", ".md"];
const RUST_ANALYZER_ID: &str = "rust-analyzer";

#[derive(Debug, Clone)]
struct CatalogEntry {
    extensions: Vec<String>,
    command: Vec<String>,
}

fn catalog(plugin_root: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    let path = plugin_root.join("lsp/server-catalog.toml");
    let data = load_toml(&path)?;
    let servers = data
        .get("servers")
        .and_then(toml::Value::as_array)
        .filter(|items| !items.is_empty())
        .with_context(|| {
            format!(
                "{} must contain [[servers]] entries",
                display_relative(&path)
            )
        })?;
    let mut known = BTreeMap::new();
    for (index, server) in servers.iter().enumerate() {
        let table = server.as_table().with_context(|| {
            format!(
                "{} servers[{}] must be a table",
                display_relative(&path),
                index + 1
            )
        })?;
        let id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .filter(|value| !value.is_empty())
            .with_context(|| {
                format!(
                    "{} servers[{}].id must be a non-empty string",
                    display_relative(&path),
                    index + 1
                )
            })?;
        if known.contains_key(id) {
            bail!("{} duplicate server id: {id}", display_relative(&path));
        }
        let extensions = toml_array_strings(table.get("extensions"))
            .filter(|items| !items.is_empty())
            .with_context(|| {
                format!(
                    "{} {id}.extensions must be a list of strings",
                    display_relative(&path)
                )
            })?;
        let command = toml_array_strings(table.get("command"))
            .filter(|items| !items.is_empty() && items.iter().all(|item| !item.is_empty()))
            .with_context(|| {
                format!(
                    "{} {id}.command must be a non-empty argv array",
                    display_relative(&path)
                )
            })?;
        if table.contains_key("args") {
            bail!(
                "{} {id}.args is not allowed; include argv in command",
                display_relative(&path)
            );
        }
        known.insert(
            id.to_owned(),
            CatalogEntry {
                extensions,
                command,
            },
        );
    }
    Ok(known)
}

fn entries(plugin_root: &Path) -> Result<BTreeMap<String, Value>> {
    let path = plugin_root.join(".codex/lsp-client.json");
    let data = load_json(&path)?;
    let entries = data
        .get("lsp")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} must contain an object at key 'lsp'",
                display_relative(&path)
            )
        })?;
    let mut output = BTreeMap::new();
    for (server_id, entry) in entries {
        if server_id.is_empty() {
            bail!(
                "{} LSP server ids must be non-empty strings",
                display_relative(&path)
            );
        }
        let object = entry.as_object().with_context(|| {
            format!("{} {server_id} must be an object", display_relative(&path))
        })?;
        let extensions = json_array_strings(object.get("extensions"))
            .filter(|items| !items.is_empty())
            .with_context(|| {
                format!(
                    "{} {server_id}.extensions must be a list of strings",
                    display_relative(&path)
                )
            })?;
        if !entry.get("priority").is_some_and(Value::is_i64) {
            bail!(
                "{} {server_id}.priority must be an integer",
                display_relative(&path)
            );
        }
        if let Some(command) = object.get("command") {
            let valid = json_array_strings(Some(command)).is_some_and(|items| {
                !items.is_empty() && items.iter().all(|item| !item.is_empty())
            });
            if !valid {
                bail!(
                    "{} {server_id}.command must be a non-empty argv array",
                    display_relative(&path)
                );
            }
        }
        if object.contains_key("args") {
            bail!(
                "{} {server_id}.args is not allowed; include argv in command",
                display_relative(&path)
            );
        }
        let mut cloned = entry.clone();
        cloned["extensions"] = Value::Array(extensions.into_iter().map(Value::String).collect());
        output.insert(server_id.to_owned(), cloned);
    }
    Ok(output)
}

pub(super) fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    let mut extensions = entries(plugin_root)?
        .values()
        .flat_map(|entry| json_array_strings(entry.get("extensions")).unwrap_or_default())
        .collect::<Vec<_>>();
    extensions.sort();
    extensions.dedup();
    Ok(extensions)
}

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match check_inner(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

pub(super) fn check_rust_readiness(plugin_root: &Path) -> Vec<String> {
    match check_rust_readiness_inner(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

fn check_inner(plugin_root: &Path) -> Result<()> {
    let entries = entries(plugin_root)?;
    let catalog = catalog(plugin_root)?;
    let mut covered = Vec::new();
    for (server_id, entry) in &entries {
        let Some(catalog_entry) = catalog.get(server_id) else {
            bail!("LSP server {server_id:?} is not present in lsp/server-catalog.toml");
        };
        let extensions = json_array_strings(entry.get("extensions")).unwrap_or_default();
        let undeclared = extensions
            .iter()
            .filter(|extension| !catalog_entry.extensions.contains(*extension))
            .cloned()
            .collect::<Vec<_>>();
        if !undeclared.is_empty() {
            bail!(
                "LSP server {server_id:?} configures extensions not declared by catalog: {}",
                undeclared.join(", ")
            );
        }
        let command = json_array_strings(entry.get("command")).unwrap_or_default();
        if command != catalog_entry.command {
            bail!(
                "LSP server {server_id:?} must define command argv {:?} from lsp/server-catalog.toml",
                catalog_entry.command
            );
        }
        covered.extend(extensions);
    }
    let missing = REQUIRED_LSP_EXTENSIONS
        .iter()
        .filter(|extension| !covered.iter().any(|item| item == **extension))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        bail!(
            "LSP coverage missing required extensions: {}",
            missing.join(", ")
        )
    }
}

fn check_rust_readiness_inner(plugin_root: &Path) -> Result<()> {
    check_inner(plugin_root)?;
    let entries = entries(plugin_root)?;
    let catalog = catalog(plugin_root)?;
    let entry = entries.get(RUST_ANALYZER_ID).with_context(
        || "Rust LSP config missing rust-analyzer entry for .rs readiness evidence",
    )?;
    let catalog_entry = catalog.get(RUST_ANALYZER_ID).with_context(
        || "Rust LSP catalog missing rust-analyzer entry for .rs readiness evidence",
    )?;
    let extensions = json_array_strings(entry.get("extensions")).unwrap_or_default();
    if !extensions.iter().any(|extension| extension == ".rs") {
        bail!("Rust LSP config must map .rs files to rust-analyzer before PR readiness");
    }
    let command =
        json_array_strings(entry.get("command")).unwrap_or_else(|| catalog_entry.command.clone());
    let command =
        crate::lsp::command::resolve_command(&command, Some(&plugin_root.display().to_string()))?;
    let (available, _, reason) = crate::lsp::command::resolve_executable(&command);
    if available {
        Ok(())
    } else {
        bail!(
            "Rust LSP command unavailable: {}; install rust-analyzer, for example with `rustup component add rust-analyzer`, or put rust-analyzer on PATH before PR readiness",
            reason.unwrap_or_else(|| "rust-analyzer executable unavailable".to_owned())
        )
    }
}
