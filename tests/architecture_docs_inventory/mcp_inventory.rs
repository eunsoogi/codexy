use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Registration {
    registration: String,
}

pub(super) fn packaged(root: &Path) -> Result<BTreeMap<String, Registration>, String> {
    let source = std::fs::read_to_string(root.join("plugins/codexy/.mcp.json"))
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&source).map_err(|error| error.to_string())?;
    let servers = value.as_object().ok_or("MCP config must be an object")?;
    servers
        .iter()
        .map(|(name, config)| {
            let registration = if let Some(url) = text(config, "url") {
                format!("Remote endpoint `{url}`.")
            } else {
                local_registration(name, config)?
            };
            Ok((name.clone(), Registration { registration }))
        })
        .collect()
}

pub(super) fn documented(guide: &str) -> Result<BTreeMap<String, Registration>, String> {
    let mut mcps = BTreeMap::new();
    for row in super::rows(guide, "MCP servers")? {
        if row.len() != 4 || row.iter().any(String::is_empty) {
            return Err(format!("MCP row must have four non-empty columns: {row:?}"));
        }
        let name = row[0].clone();
        let registration = Registration { registration: row[1].clone() };
        if mcps.insert(name.clone(), registration).is_some() {
            return Err(format!("duplicate documented MCP: {name}"));
        }
    }
    Ok(mcps)
}

fn local_registration(name: &str, config: &serde_json::Value) -> Result<String, String> {
    let command = text(config, "command").ok_or_else(|| format!("MCP {name} command missing"))?;
    let args = config
        .get("args")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("MCP {name} args missing"))?
        .iter()
        .map(|arg| arg.as_str().ok_or_else(|| format!("MCP {name} arg must be text")))
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    let cwd = text(config, "cwd").ok_or_else(|| format!("MCP {name} cwd missing"))?;
    Ok(format!("Plugin-relative `{command} {args}`; cwd `{cwd}`."))
}

fn text<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_json::Value::as_str)
}
