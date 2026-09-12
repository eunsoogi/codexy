use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

const CONFIGS: [(&str, &[&str]); 2] = [
    ("plugins/codexy/.mcp.json", &["watcher"]),
    ("plugins/codexy-devtools/.mcp.json", &["lsp", "codegraph"]),
];
const BOOTSTRAP: &str = "./mcp/codexy_mcp_bootstrap.py";

pub(super) fn check_at(root: &Path) -> Result<()> {
    let present = CONFIGS
        .iter()
        .filter(|(relative, _)| root.join(relative).is_file())
        .count();
    if present == 0 {
        return Ok(());
    }
    if present != CONFIGS.len() {
        bail!("MCP bootstrap configuration is only partially present");
    }
    for (relative, servers) in CONFIGS {
        let path = root.join(relative);
        let value = super::load_json(&path)?;
        let object = value.as_object().context("MCP config must be an object")?;
        if object.len() != servers.len()
            || servers
                .iter()
                .any(|server| !matches_entry(object.get(*server), server))
        {
            bail!(
                "{} must use the shared metadata-driven MCP bootstrap",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

fn bootstrap_args(server: &str) -> Vec<String> {
    vec![
        "run".to_owned(),
        "--no-project".to_owned(),
        "--script".to_owned(),
        BOOTSTRAP.to_owned(),
        server.to_owned(),
        "--stdio".to_owned(),
    ]
}

fn matches_entry(entry: Option<&Value>, server: &str) -> bool {
    let Some(entry) = entry.and_then(Value::as_object) else {
        return false;
    };
    entry.len() == 3
        && entry.get("command") == Some(&Value::String("uv".to_owned()))
        && entry.get("cwd") == Some(&Value::String(".".to_owned()))
        && entry.get("args")
            == Some(&Value::Array(
                bootstrap_args(server)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ))
}
