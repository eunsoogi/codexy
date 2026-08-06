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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{Map, Value};

    use super::check;

    #[test]
    fn cross_host_entrypoint_matrix_preserves_exact_diagnostics() {
        let path = Path::new(".mcp.json");
        let mut object = Map::new();
        object.insert("cwd".to_owned(), Value::String(".".to_owned()));
        let expected = format!(
            ".mcp.json lsp.command must use the exact cross-platform plugin entrypoint {:?}",
            ["./mcp/codexy-mcp-lsp", "--stdio"]
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
