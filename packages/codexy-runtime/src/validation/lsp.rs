use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_toml};

const REQUIRED_LSP_EXTENSIONS: &[&str] = &[
    ".py", ".pyi", ".yaml", ".yml", ".json", ".toml", ".md", ".html", ".css", ".scss", ".less",
    ".graphql", ".gql",
];
const SMOKE_SERVER_EXTENSIONS: &[(&str, &str)] = &[
    ("rust-analyzer", ".rs"),
    ("basedpyright", ".py"),
    ("yaml-ls", ".yaml"),
    ("json-language-server", ".json"),
    ("taplo", ".toml"),
    ("marksman", ".md"),
    ("html-language-server", ".html"),
    ("css-language-server", ".css"),
    ("graphql-language-service", ".graphql"),
];
const JSON_FIELDS: &[&str] = &["extensions", "priority", "command"];
macro_rules! fail_with {
    ($path:expr, $code:literal, $message:literal $(, $args:expr)*) => {
        bail!(concat!($code, ": {} ", $message), display_relative($path) $(, $args)*);
    };
}

macro_rules! drift {
    ($id:expr, $actual:expr, $expected:expr, $field:ident, $label:literal) => {
        if $actual.$field != $expected.$field {
            bail!(
                "PROJECTION_DRIFT: {} {}.{}",
                $label,
                $id,
                stringify!($field)
            );
        }
    };
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogEntry {
    #[serde(skip_serializing)]
    id: String,
    #[serde(skip_serializing)]
    language: String,
    extensions: Vec<String>,
    priority: i64,
    command: Vec<String>,
    #[serde(skip_serializing)]
    install: String,
}

#[derive(Debug, Deserialize)]
struct CatalogFile {
    servers: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JsonEntry {
    extensions: Vec<String>,
    priority: i64,
    command: Vec<String>,
}

fn valid_array(value: Option<&Value>, command: bool) -> bool {
    json_array_strings(value).is_some_and(|items| {
        !items.is_empty() && (!command || items.iter().all(|item| !item.is_empty()))
    })
}

fn catalog(plugin_root: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    let path = plugin_root.join("lsp/server-catalog.toml");
    let file: CatalogFile = load_toml(&path)?
        .try_into()
        .map_err(|error| anyhow::anyhow!("PROJECTION_DRIFT: invalid catalog: {error}"))?;
    if file.servers.is_empty() {
        fail_with!(&path, "ID_SET_MISMATCH", "must contain servers entries");
    }
    let mut known = BTreeMap::new();
    for server in file.servers {
        let id = server.id.clone();
        if id.is_empty() || server.language.is_empty() || server.install.is_empty() {
            fail_with!(&path, "PROJECTION_DRIFT", "required catalog field empty");
        }
        if server.extensions.is_empty() {
            fail_with!(&path, "PROJECTION_DRIFT", "{}.extensions is empty", id);
        }
        if server.command.is_empty() || server.command.iter().any(String::is_empty) {
            fail_with!(&path, "EMPTY_COMMAND", "{}.command invalid or empty", id);
        }
        if known.insert(id.clone(), server).is_some() {
            fail_with!(&path, "DUPLICATE_ID", "duplicate server id: {}", id);
        }
    }
    Ok(known)
}

fn entries(plugin_root: &Path) -> Result<(BTreeMap<String, JsonEntry>, String)> {
    let path = plugin_root.join(".codex/lsp-client.json");
    let text = fs::read_to_string(&path)
        .map_err(|error| anyhow::anyhow!("PROJECTION_DRIFT: read lsp JSON: {error}"))?;
    let data: Value = serde_json::from_str(&text)
        .map_err(|error| anyhow::anyhow!("PROJECTION_DRIFT: invalid lsp JSON: {error}"))?;
    let object = data
        .get("lsp")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("PROJECTION_DRIFT: missing lsp object"))?;
    for (server_id, entry) in object {
        if server_id.is_empty() {
            fail_with!(&path, "PROJECTION_DRIFT", "LSP server id empty");
        }
        let object = entry
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("PROJECTION_DRIFT: {server_id} is not an object"))?;
        let unknown = object
            .keys()
            .find(|key| !JSON_FIELDS.contains(&key.as_str()));
        if let Some(key) = unknown {
            fail_with!(&path, "UNSUPPORTED_JSON_KEY", "{}.{}", server_id, key);
        }
        if !valid_array(object.get("extensions"), false) {
            fail_with!(&path, "PROJECTION_DRIFT", "bad extensions {}", server_id);
        }
        if !valid_array(object.get("command"), true) {
            fail_with!(&path, "EMPTY_COMMAND", "bad command {}", server_id);
        }
    }
    let output = serde_json::from_value(Value::Object(object.clone()))
        .map_err(|error| anyhow::anyhow!("PROJECTION_DRIFT: invalid projection fields: {error}"))?;
    Ok((output, text))
}

fn projection(catalog: &BTreeMap<String, CatalogEntry>) -> Value {
    let lsp = catalog
        .iter()
        .map(|(id, entry)| (id.clone(), json!(entry)))
        .collect::<Map<_, _>>();
    json!({"lsp": lsp})
}

fn covered<'a>(entries: impl Iterator<Item = &'a JsonEntry>) -> Vec<String> {
    let mut extensions = entries
        .flat_map(|entry| entry.extensions.iter().cloned())
        .collect::<Vec<_>>();
    extensions.sort();
    extensions.dedup();
    extensions
}

pub(super) fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    Ok(covered(entries(plugin_root)?.0.values()))
}

fn diagnostics(result: Result<()>) -> Vec<String> {
    result.map_or_else(|error| vec![error.to_string()], |_| Vec::new())
}

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    diagnostics(check_inner(plugin_root))
}

pub(super) fn check_rust_readiness(plugin_root: &Path) -> Vec<String> {
    diagnostics(check_rust_readiness_inner(plugin_root))
}

fn check_inner(plugin_root: &Path) -> Result<()> {
    let catalog = catalog(plugin_root)?;
    if catalog.len() != 39 {
        bail!(
            "ID_SET_MISMATCH: catalog contains {}, expected 39 IDs (30 lazy)",
            catalog.len()
        );
    }
    for (id, extension) in SMOKE_SERVER_EXTENSIONS {
        let Some(entry) = catalog.get(*id) else {
            bail!("ID_SET_MISMATCH: smoke server {id} is missing from catalog");
        };
        if !entry.extensions.iter().any(|item| item == extension) {
            bail!("SMOKE_EXTENSION_MISSING: {id} must declare {extension}");
        }
    }

    let (entries, _text) = entries(plugin_root)?;
    if let Some(id) = entries.keys().find(|id| !catalog.contains_key(*id)) {
        bail!("UNKNOWN_JSON_ID: {id}");
    }
    if entries.len() != catalog.len() || catalog.keys().any(|id| !entries.contains_key(id)) {
        bail!("ID_SET_MISMATCH: TOML and JSON server IDs differ");
    }
    let covered = covered(entries.values());
    let missing = REQUIRED_LSP_EXTENSIONS
        .iter()
        .filter(|extension| !covered.iter().any(|item| item == **extension))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "SMOKE_EXTENSION_MISSING: LSP coverage missing required extensions: {}",
            missing.join(", ")
        )
    }
    for (id, expected) in &catalog {
        let Some(actual) = entries.get(id) else {
            bail!("ID_SET_MISMATCH: TOML and JSON server IDs differ");
        };
        drift!(id, actual, expected, extensions, "EXTENSION_DRIFT");
        drift!(id, actual, expected, priority, "PRIORITY_DRIFT");
        drift!(id, actual, expected, command, "COMMAND_DRIFT");
    }
    if json!({"lsp": entries}) != projection(&catalog) {
        bail!("PROJECTION_DRIFT: JSON is not the deterministic sorted projection");
    }

    Ok(())
}

fn check_rust_readiness_inner(plugin_root: &Path) -> Result<()> {
    check_inner(plugin_root)?;
    let (entries, _) = entries(plugin_root)?;
    let entry = entries.get("rust-analyzer").with_context(
        || "Rust LSP config missing rust-analyzer entry for .rs readiness evidence",
    )?;
    if !entry.extensions.iter().any(|extension| extension == ".rs") {
        bail!("SMOKE_EXTENSION_MISSING: Rust LSP config must map .rs files to rust-analyzer");
    }
    let command = &entry.command;
    let command =
        crate::lsp::command::resolve_command(command, Some(&plugin_root.display().to_string()))?;
    let (available, _, reason) = crate::lsp::command::resolve_executable(&command);
    if available {
        Ok(())
    } else {
        bail!(
            "Rust LSP command unavailable: {}; install rust-analyzer, for example with \x60rustup component add rust-analyzer\x60, or put rust-analyzer on PATH before PR readiness",
            reason.unwrap_or_else(|| "rust-analyzer executable unavailable".to_owned())
        )
    }
}
