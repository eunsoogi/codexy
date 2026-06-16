use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lsp::command::{resolve_command, resolve_executable};
use crate::lsp::pathing::normalize_ext;
use crate::paths::plugin_root;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct LspEntry {
    pub(super) extensions: Vec<String>,
    pub(super) priority: i64,
    pub(super) command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Server {
    pub(super) id: String,
    pub(super) language: Option<String>,
    pub(super) extensions: Vec<String>,
    pub(super) command: Option<Vec<String>>,
    pub(super) executable: Option<String>,
    #[serde(rename = "resolvedExecutable")]
    pub(super) resolved_executable: Option<String>,
    pub(super) available: bool,
    #[serde(rename = "installHints")]
    pub(super) install_hints: Vec<String>,
    #[serde(rename = "unavailableReason", skip_serializing_if = "Option::is_none")]
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalog {
    servers: Vec<CatalogServer>,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogServer {
    id: String,
    language: Option<String>,
    command: Vec<String>,
    install: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LspClientConfig {
    lsp: BTreeMap<String, LspEntry>,
}

pub(super) fn read_config() -> Result<BTreeMap<String, LspEntry>> {
    let text = fs::read_to_string(plugin_root().join(".codex/lsp-client.json"))?;
    Ok(serde_json::from_str::<LspClientConfig>(&text)?.lsp)
}

pub(super) fn matching_servers(file_path: &str, root: Option<&str>) -> Result<Vec<Server>> {
    let extension = normalize_ext(file_path);
    let catalog = read_catalog()?;
    let mut matches = read_config()?
        .into_iter()
        .filter(|(_, entry)| entry.extensions.iter().any(|item| item == &extension))
        .map(|(id, entry)| {
            let priority = entry.priority;
            let server = enrich_server(&id, &entry, &catalog, root)?;
            Ok((priority, server))
        })
        .collect::<Result<Vec<_>>>()?;
    matches.sort_by_key(|item| Reverse(item.0));
    Ok(matches.into_iter().map(|(_, server)| server).collect())
}

pub(super) fn select_server(args: &Value, file_path: &str, root: Option<&str>) -> Result<Server> {
    if let Some(override_value) = args.get("server").filter(|value| !value.is_null()) {
        return server_from_override(override_value, root);
    }
    let matches = matching_servers(file_path, root)?;
    if let Some(server) = matches.into_iter().next() {
        return Ok(server);
    }
    let extension_label = normalize_ext(file_path);
    let unmatched = if extension_label.is_empty() {
        Path::new(file_path)
            .file_name()
            .and_then(|item| item.to_str())
            .unwrap_or("")
            .to_owned()
    } else {
        extension_label
    };
    Ok(Server {
        id: "unmatched".to_owned(),
        language: None,
        extensions: Vec::new(),
        command: None,
        executable: None,
        resolved_executable: None,
        available: false,
        install_hints: Vec::new(),
        reason: Some(format!("no LSP server matches {unmatched}")),
    })
}

fn read_catalog() -> Result<BTreeMap<String, CatalogServer>> {
    let text = fs::read_to_string(plugin_root().join("lsp/server-catalog.toml"))?;
    let catalog = toml::from_str::<Catalog>(&text)?;
    Ok(catalog
        .servers
        .into_iter()
        .map(|server| (server.id.clone(), server))
        .collect())
}

fn server_from_override(value: &Value, root: Option<&str>) -> Result<Server> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|item| !item.is_empty())
        .context("server.id is required when server override is provided")?;
    let command_override = value.get("command").and_then(json_string_array);
    let overrides_allowed = std::env::var("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE")
        .ok()
        .as_deref()
        == Some("1");
    if command_override.is_some() && !overrides_allowed {
        return Ok(unavailable_override(
            id,
            command_override,
            "server command overrides require CODEXY_LSP_ALLOW_COMMAND_OVERRIDE=1",
        ));
    }
    let config = read_config()?;
    let catalog = read_catalog()?;
    if !config.contains_key(id) && !catalog.contains_key(id) && !overrides_allowed {
        return Ok(unavailable_override(
            id,
            None,
            &format!("server id is not configured or cataloged: {id}"),
        ));
    }
    let mut entry = config.get(id).cloned().unwrap_or(LspEntry {
        extensions: Vec::new(),
        priority: 0,
        command: None,
    });
    if let Some(command) = command_override {
        entry.command = Some(command);
    }
    enrich_server(id, &entry, &catalog, root)
}

fn enrich_server(
    id: &str,
    entry: &LspEntry,
    catalog: &BTreeMap<String, CatalogServer>,
    root: Option<&str>,
) -> Result<Server> {
    let catalog_entry = catalog.get(id);
    let command = entry
        .command
        .clone()
        .or_else(|| catalog_entry.map(|item| item.command.clone()));
    let command = command
        .map(|items| resolve_command(&items, root))
        .transpose()?;
    let availability = command.as_ref().map_or_else(
        || (false, None, Some("server command is missing".to_owned())),
        |items| resolve_executable(items),
    );
    Ok(Server {
        id: id.to_owned(),
        language: catalog_entry.and_then(|item| item.language.clone()),
        extensions: entry.extensions.clone(),
        executable: command.as_ref().and_then(|items| items.first().cloned()),
        command,
        resolved_executable: availability.1,
        available: availability.0,
        install_hints: catalog_entry
            .and_then(|item| item.install.clone())
            .into_iter()
            .collect(),
        reason: availability.2,
    })
}

fn unavailable_override(id: &str, command: Option<Vec<String>>, reason: &str) -> Server {
    Server {
        id: id.to_owned(),
        language: None,
        extensions: Vec::new(),
        executable: command.as_ref().and_then(|items| items.first().cloned()),
        command,
        resolved_executable: None,
        available: false,
        install_hints: Vec::new(),
        reason: Some(reason.to_owned()),
    }
}

fn json_string_array(value: &Value) -> Option<Vec<String>> {
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect()
    })
}
