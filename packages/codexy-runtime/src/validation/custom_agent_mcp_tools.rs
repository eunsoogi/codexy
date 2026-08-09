use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const ALLOWED_TOOL_FIELDS: &[&str] = &["approval_mode"];
const APPROVAL_MODES: &[&str] = &["auto", "prompt", "approve"];

pub(super) fn check(path: &Path, server: &str, value: Option<&Value>, errors: &mut Vec<String>) {
    let Some(value) = value else {
        return;
    };
    let Value::Table(tools) = value else {
        errors.push(format!(
            "{} mcp_servers.{server}.tools must be a table",
            display_relative(path)
        ));
        return;
    };
    for (tool, value) in tools {
        check_tool(path, server, tool, value, errors);
    }
}

fn check_tool(path: &Path, server: &str, tool: &str, value: &Value, errors: &mut Vec<String>) {
    let Value::Table(fields) = value else {
        errors.push(format!(
            "{} mcp_servers.{server}.tools.{tool} must be a table",
            display_relative(path)
        ));
        return;
    };
    for key in fields.keys() {
        if !ALLOWED_TOOL_FIELDS.contains(&key.as_str()) {
            errors.push(format!(
                "{} mcp_servers.{server}.tools.{tool}.{key} is not part of the supported Codex MCP tool override schema",
                display_relative(path)
            ));
        }
    }
    if fields.get("approval_mode").is_some_and(|value| {
        !value
            .as_str()
            .is_some_and(|mode| APPROVAL_MODES.contains(&mode))
    }) {
        errors.push(format!(
            "{} mcp_servers.{server}.tools.{tool}.approval_mode has an unsupported value",
            display_relative(path)
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use toml::Value;

    use super::check;

    #[test]
    fn invalid_tools_matrix_preserves_exact_diagnostics() {
        let path = Path::new("agents/codexy-pathfinder.toml");
        for (fragment, expected) in [
            (
                "\n[mcp_servers.example_mcp]\ncommand = \"example_mcp\"\ntools = \"bad\"\n",
                "agents/codexy-pathfinder.toml mcp_servers.example_mcp.tools must be a table",
            ),
            (
                "\n[mcp_servers.docs]\ncommand = \"docs\"\n[mcp_servers.docs.tools.search]\napproval_mode = \"always\"\n",
                "agents/codexy-pathfinder.toml mcp_servers.docs.tools.search.approval_mode has an unsupported value",
            ),
            (
                "\n[mcp_servers.docs]\ncommand = \"docs\"\n[mcp_servers.docs.tools.search]\nunknown = true\n",
                "agents/codexy-pathfinder.toml mcp_servers.docs.tools.search.unknown is not part of the supported Codex MCP tool override schema",
            ),
        ] {
            let parsed = toml::from_str::<toml::Table>(fragment).expect("valid TOML fragment");
            let servers = parsed
                .get("mcp_servers")
                .and_then(Value::as_table)
                .expect("MCP server table");
            let (server, entry) = servers.iter().next().expect("MCP server");
            let mut errors = Vec::new();
            check(path, server, entry.get("tools"), &mut errors);
            assert_eq!(errors, vec![expected.to_owned()], "input: {fragment:?}");
        }
    }
}
