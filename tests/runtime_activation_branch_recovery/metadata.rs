use std::{path::Path, fs};

use serde_json::{Value, json};

pub(super) fn restore_legacy_platform_metadata(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = repo.join("plugins/codexy/.codex-plugin/plugin.json");
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] = json!(["darwin-arm64", "linux-x86_64"]);
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;

    let marketplace_path = repo.join(".agents/plugins/marketplace.json");
    let mut marketplace: Value = serde_json::from_str(&fs::read_to_string(&marketplace_path)?)?;
    marketplace["plugins"][0]["platforms"] = json!(["darwin-arm64", "linux-x86_64"]);
    fs::write(&marketplace_path, format!("{}\n", serde_json::to_string_pretty(&marketplace)?))?;
    Ok(())
}
