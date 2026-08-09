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
            is_removed_name(text) || is_removed_command(text) || is_removed_url(text)
        }
        Value::Array(values) => values.iter().any(contains_removed_reference),
        Value::Table(values) => values.values().any(contains_removed_reference),
        Value::Boolean(_) | Value::Datetime(_) | Value::Float(_) | Value::Integer(_) => false,
    }
}

fn is_removed_command(text: &str) -> bool {
    text.trim()
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(is_removed_name)
}

fn is_removed_url(text: &str) -> bool {
    let authority = text
        .trim()
        .split_once("://")
        .map_or(text.trim(), |(_, value)| value)
        .trim_start_matches("//")
        .rsplit('@')
        .next()
        .unwrap_or_default();
    authority
        .split(['/', '?', '#', ':'])
        .next()
        .map(|host| host.trim_end_matches('.'))
        .is_some_and(|host| {
            matches!(
                host.to_ascii_lowercase().as_str(),
                "grep.app" | "mcp.grep.app"
            )
        })
}
