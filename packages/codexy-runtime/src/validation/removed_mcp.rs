use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

pub(super) fn is_removed_name(name: &str) -> bool {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .eq_ignore_ascii_case("grepapp")
}

pub(super) fn check_custom_agent(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    errors: &mut Vec<String>,
) {
    if is_removed_name(name) {
        errors.push(format!(
            "{} mcp_servers.{name} references removed MCP server",
            display_relative(path)
        ));
    }
    if fields.values().any(contains_removed_reference) {
        errors.push(format!(
            "{} mcp_servers.{name} references removed MCP endpoint or command",
            display_relative(path)
        ));
    }
}

fn contains_removed_reference(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            is_removed_name(text) || text.to_ascii_lowercase().contains("mcp.grep.app")
        }
        Value::Array(values) => values.iter().any(contains_removed_reference),
        Value::Table(values) => values.values().any(contains_removed_reference),
        Value::Boolean(_) | Value::Datetime(_) | Value::Float(_) | Value::Integer(_) => false,
    }
}
