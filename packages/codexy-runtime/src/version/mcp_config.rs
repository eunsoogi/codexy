use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::mcp_required::is_shared_bootstrap_entry;

const CONFIGS: [(&str, &[&str]); 2] = [
    ("plugins/codexy/.mcp.json", &["watcher"]),
    ("plugins/codexy-devtools/.mcp.json", &["lsp", "codegraph"]),
];
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
            || servers.iter().any(|server| {
                !is_shared_bootstrap_entry(object.get(*server).and_then(Value::as_object), server)
            })
        {
            bail!(
                "{} must use the shared metadata-driven MCP bootstrap",
                display_relative(&path)
            );
        }
    }
    Ok(())
}
