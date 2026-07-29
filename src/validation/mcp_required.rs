use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::paths::display_relative;

pub(super) fn check(
    path: &Path,
    name: &str,
    object: &Map<String, Value>,
    command: &[String],
) -> Result<()> {
    if !matches!(name, "lsp" | "codegraph") {
        return Ok(());
    }
    let expected = [format!("./mcp/codexy-mcp-{name}"), "--stdio".to_string()];
    if command != expected {
        bail!(
            "{} {name}.command must use the exact cross-platform plugin entrypoint {:?}",
            display_relative(path),
            expected
        );
    }
    if object.get("cwd").and_then(Value::as_str) != Some(".") {
        bail!(
            "{} {name}.cwd must be '.' so Codex resolves the command from the plugin root",
            display_relative(path)
        );
    }
    Ok(())
}
