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
    if !matches!(name, "watcher" | "lsp" | "codegraph") {
        return Ok(());
    }
    let expected = shared_bootstrap_command(name);
    if command != expected {
        bail!(
            "{} {name}.command must use the exact shared cross-platform MCP bootstrap {:?}",
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

pub(crate) fn shared_bootstrap_command(name: &str) -> Vec<String> {
    vec![
        "uv".to_owned(),
        "run".to_owned(),
        "--no-project".to_owned(),
        "--script".to_owned(),
        "./mcp/codexy_mcp_bootstrap.py".to_owned(),
        name.to_owned(),
        "--stdio".to_owned(),
    ]
}

pub(crate) fn is_shared_bootstrap(command: &[String]) -> bool {
    ["watcher", "lsp", "codegraph"]
        .iter()
        .any(|name| command == shared_bootstrap_command(name))
}

pub(crate) fn is_shared_bootstrap_entry(
    entry: Option<&serde_json::Map<String, Value>>,
    name: &str,
) -> bool {
    let Some(entry) = entry else {
        return false;
    };
    let Some(command) = entry.get("command").and_then(Value::as_str) else {
        return false;
    };
    let Some(args) = entry.get("args").and_then(Value::as_array) else {
        return false;
    };
    let Some(args) = args.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
        return false;
    };
    let mut command_items = vec![command.to_owned()];
    command_items.extend(args.into_iter().map(ToOwned::to_owned));
    entry.len() == 3
        && entry.get("cwd") == Some(&Value::String(".".to_owned()))
        && command_items == shared_bootstrap_command(name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{Map, Value};

    use super::{check, shared_bootstrap_command};

    #[test]
    fn cross_host_entrypoint_matrix_preserves_exact_diagnostics() {
        let path = Path::new(".mcp.json");
        let mut object = Map::new();
        object.insert("cwd".to_owned(), Value::String(".".to_owned()));
        let expected = format!(
            ".mcp.json lsp.command must use the exact shared cross-platform MCP bootstrap {:?}",
            shared_bootstrap_command("lsp")
        );

        for command in [
            "python3.exe",
            r"C:\tools\codexy-mcp-lsp.exe",
            "C:/tools/codexy-mcp-lsp.exe",
            r".\..\outside.exe",
            r"\\server\share\codexy-mcp-lsp.exe",
            r"\\?\C:\codexy-mcp-lsp.exe",
        ] {
            let command = vec![command.to_owned(), "--stdio".to_owned()];
            let error = check(path, "lsp", &object, &command)
                .expect_err("noncanonical cross-host entrypoint unexpectedly passed")
                .to_string();
            assert_eq!(error, expected, "unexpected diagnostic for {command:?}");
        }
    }
}
