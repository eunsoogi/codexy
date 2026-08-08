use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Registration {
    config: serde_json::Value,
}

pub(super) fn packaged(root: &Path) -> Result<BTreeMap<String, Registration>, String> {
    let source = std::fs::read_to_string(root.join("plugins/codexy/.mcp.json"))
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&source).map_err(|error| error.to_string())?;
    let servers = value.as_object().ok_or("MCP config must be an object")?;
    servers
        .iter()
        .map(|(name, config)| Ok((name.clone(), registration(name, config.clone())?)))
        .collect()
}

pub(super) fn documented(guide: &str) -> Result<BTreeMap<String, Registration>, String> {
    let mut mcps = BTreeMap::new();
    for row in super::rows(guide, "MCP servers")? {
        if row.len() != 4 || row.iter().any(String::is_empty) {
            return Err(format!("MCP row must have four non-empty columns: {row:?}"));
        }
        let name = row[0].clone();
        let config = serde_json::from_str(&row[1])
            .map_err(|error| format!("documented MCP {name} registration must be JSON: {error}"))?;
        let registration = registration(&name, config)?;
        if mcps.insert(name.clone(), registration).is_some() {
            return Err(format!("duplicate documented MCP: {name}"));
        }
    }
    Ok(mcps)
}

fn registration(name: &str, config: serde_json::Value) -> Result<Registration, String> {
    if !config.is_object() {
        return Err(format!("MCP {name} registration must be an object"));
    }
    Ok(Registration { config })
}

#[test]
fn packaged_registrations_preserve_argument_boundaries() -> Result<(), String> {
    let single = fixture(serde_json::json!({
        "command": "./mcp/server",
        "args": ["--flag value"],
        "cwd": "."
    }))?;
    let split = fixture(serde_json::json!({
        "command": "./mcp/server",
        "args": ["--flag", "value"],
        "cwd": "."
    }))?;
    assert_ne!(single, split);
    Ok(())
}

#[test]
fn packaged_registrations_preserve_url_and_local_fields_together() -> Result<(), String> {
    let remote = fixture(serde_json::json!({"url": "https://mcp.example"}))?;
    let combined = fixture(serde_json::json!({
        "url": "https://mcp.example",
        "command": "./mcp/server",
        "args": ["--stdio"],
        "cwd": "."
    }))?;
    assert_ne!(remote, combined);
    Ok(())
}

fn fixture(config: serde_json::Value) -> Result<BTreeMap<String, Registration>, String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    let plugin = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin).map_err(|error| error.to_string())?;
    let source = serde_json::to_string(&serde_json::json!({"fixture": config}))
        .map_err(|error| error.to_string())?;
    std::fs::write(plugin.join(".mcp.json"), source).map_err(|error| error.to_string())?;
    packaged(root.path())
}
