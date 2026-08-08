use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

pub(super) fn check(catalog_path: &Path, catalog: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    require_value(
        catalog_path,
        catalog,
        "native_custom_agent_registration",
        "codex-home-standalone-agent-projection",
        &mut errors,
    );
    require_value(
        catalog_path,
        catalog,
        "native_custom_agent_projection",
        "managed-codexy-subdirectory",
        &mut errors,
    );
    errors
}

fn require_value(
    path: &Path,
    catalog: &Value,
    key: &str,
    expected: &str,
    errors: &mut Vec<String>,
) {
    if catalog.get(key).and_then(Value::as_str) != Some(expected) {
        errors.push(format!(
            "{} {key} must be {expected}",
            display_relative(path)
        ));
    }
}
