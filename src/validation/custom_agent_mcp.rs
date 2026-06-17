use std::path::Path;

use toml::Value;

use crate::paths::display_relative;
use crate::validation::custom_agent_mcp_tools;

const ALLOWED_FIELDS: &[&str] = &[
    "args",
    "bearer_token_env_var",
    "command",
    "cwd",
    "default_tools_approval_mode",
    "disabled_tools",
    "enabled",
    "enabled_tools",
    "env",
    "env_http_headers",
    "env_vars",
    "experimental_environment",
    "http_headers",
    "oauth_resource",
    "required",
    "scopes",
    "startup_timeout_ms",
    "startup_timeout_sec",
    "tool_timeout_sec",
    "tools",
    "url",
];

pub(super) fn check(path: &Path, value: Option<&Value>, errors: &mut Vec<String>) {
    let Some(value) = value else {
        return;
    };
    let Value::Table(servers) = value else {
        errors.push(format!(
            "{} mcp_servers must be a table",
            display_relative(path)
        ));
        return;
    };
    for (name, entry) in servers {
        check_server(path, name, entry, errors);
    }
}

fn check_server(path: &Path, name: &str, value: &Value, errors: &mut Vec<String>) {
    let Value::Table(fields) = value else {
        errors.push(format!(
            "{} mcp_servers.{name} must be a table",
            display_relative(path)
        ));
        return;
    };
    for key in fields.keys() {
        if !ALLOWED_FIELDS.contains(&key.as_str()) {
            errors.push(format!(
                "{} mcp_servers.{name}.{key} is not part of the supported Codex MCP server schema",
                display_relative(path)
            ));
        }
    }
    if !fields.contains_key("command") && !fields.contains_key("url") {
        errors.push(format!(
            "{} mcp_servers.{name} must define command or url",
            display_relative(path)
        ));
    }
    for key in [
        "command",
        "url",
        "cwd",
        "bearer_token_env_var",
        "oauth_resource",
    ] {
        check_string(path, name, fields, key, errors);
    }
    for key in ["enabled", "required"] {
        check_bool(path, name, fields, key, errors);
    }
    for key in ["args", "enabled_tools", "disabled_tools", "scopes"] {
        check_string_array(path, name, fields, key, errors);
    }
    check_env_vars(path, name, fields, errors);
    for key in ["env", "env_http_headers", "http_headers"] {
        check_string_map(path, name, fields, key, errors);
    }
    custom_agent_mcp_tools::check(path, name, fields.get("tools"), errors);
    for key in [
        "startup_timeout_ms",
        "startup_timeout_sec",
        "tool_timeout_sec",
    ] {
        check_number(path, name, fields, key, errors);
    }
    check_enum(
        path,
        name,
        fields,
        "default_tools_approval_mode",
        &["auto", "prompt", "approve"],
        errors,
    );
    check_enum(
        path,
        name,
        fields,
        "experimental_environment",
        &["local", "remote"],
        errors,
    );
}

fn check_string(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) {
    if fields.get(key).is_some_and(|value| !value.is_str()) {
        errors.push(format!(
            "{} mcp_servers.{name}.{key} must be a string",
            display_relative(path)
        ));
    }
}

fn check_bool(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) {
    if fields.get(key).is_some_and(|value| !value.is_bool()) {
        errors.push(format!(
            "{} mcp_servers.{name}.{key} must be a boolean",
            display_relative(path)
        ));
    }
}

fn check_string_array(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) {
    if fields.get(key).is_some_and(|value| {
        !value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_str))
    }) {
        errors.push(format!(
            "{} mcp_servers.{name}.{key} must be a list of strings",
            display_relative(path)
        ));
    }
}

fn check_env_vars(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    errors: &mut Vec<String>,
) {
    if fields.get("env_vars").is_some_and(|value| {
        !value.as_array().is_some_and(|items| {
            items.iter().all(|item| {
                item.is_str()
                    || item.as_table().is_some_and(|table| {
                        table.keys().all(|key| key == "name" || key == "source")
                            && table.get("name").is_some_and(Value::is_str)
                            && table.get("source").is_none_or(|source| {
                                source
                                    .as_str()
                                    .is_some_and(|item| matches!(item, "local" | "remote"))
                            })
                    })
            })
        })
    }) {
        errors.push(format!(
            "{} mcp_servers.{name}.env_vars must list strings or name/source tables",
            display_relative(path)
        ));
    }
}

fn check_string_map(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) {
    if fields.get(key).is_some_and(|value| {
        !value
            .as_table()
            .is_some_and(|items| items.values().all(Value::is_str))
    }) {
        errors.push(format!(
            "{} mcp_servers.{name}.{key} must be a string map",
            display_relative(path)
        ));
    }
}

fn check_number(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    errors: &mut Vec<String>,
) {
    if fields
        .get(key)
        .is_some_and(|value| !(value.is_integer() || value.is_float()))
    {
        errors.push(format!(
            "{} mcp_servers.{name}.{key} must be a number",
            display_relative(path)
        ));
    }
}

fn check_enum(
    path: &Path,
    name: &str,
    fields: &toml::map::Map<String, Value>,
    key: &str,
    allowed: &[&str],
    errors: &mut Vec<String>,
) {
    if let Some(value) = fields.get(key) {
        if !value.as_str().is_some_and(|item| allowed.contains(&item)) {
            errors.push(format!(
                "{} mcp_servers.{name}.{key} has an unsupported value",
                display_relative(path)
            ));
        }
    }
}
