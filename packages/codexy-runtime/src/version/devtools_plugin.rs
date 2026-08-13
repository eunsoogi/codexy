use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{
    MARKETPLACE, fields, load_json, marketplace_plugin_mut_named, repo_path,
    require_matching_version, string_field, write_json,
};

const NAME: &str = "codexy-devtools";
const MANIFEST: &str = "plugins/codexy-devtools/.codex-plugin/plugin.json";
const SOURCE: &str = "./plugins/codexy-devtools";

pub(super) fn check(core_version: &str) -> Result<()> {
    let Some((manifest_version, marketplace_version)) = validated_versions()? else {
        return Ok(());
    };
    require_matching_version(
        &manifest_version,
        MANIFEST,
        core_version,
        "core plugin manifest",
    )?;
    require_matching_version(
        &marketplace_version,
        MARKETPLACE,
        core_version,
        "core plugin manifest",
    )
}

pub(super) fn validate_mutation_inputs() -> Result<()> {
    validated_versions().map(|_| ())
}

fn validated_versions() -> Result<Option<(String, String)>> {
    let manifest_path = repo_path(MANIFEST)?;
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let marketplace_path = repo_path(MARKETPLACE)?;
    let manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&marketplace_path)?;
    if string_field(&manifest, "name", "Devtools plugin manifest")? != NAME {
        bail!("Devtools plugin manifest name must be {NAME:?}");
    }
    let manifest_version =
        string_field(&manifest, "version", "Devtools plugin manifest")?.to_owned();
    super::require_semver(&manifest_version)?;
    if string_field(&manifest, "skills", "Devtools plugin manifest")? != "./skills/" {
        bail!("Devtools plugin manifest skills must be ./skills/");
    }
    if string_field(&manifest, "mcpServers", "Devtools plugin manifest")? != "./.mcp.json" {
        bail!("Devtools plugin manifest mcpServers must be ./.mcp.json");
    }
    let entry = marketplace_plugin_mut_named(&mut marketplace, NAME)?;
    let marketplace_version =
        string_field(entry, "version", "Devtools marketplace entry")?.to_owned();
    super::require_semver(&marketplace_version)?;
    if entry.pointer("/source/path").and_then(Value::as_str) != Some(SOURCE) {
        bail!("Devtools marketplace source must be {SOURCE:?}");
    }
    if fields::string_array(&manifest, "supportedPlatforms", "Devtools plugin manifest")?
        != fields::string_array(entry, "supportedPlatforms", "Devtools marketplace entry")?
    {
        bail!("Devtools plugin supportedPlatforms must match its marketplace entry");
    }
    let root = manifest_path
        .parent()
        .context("Devtools manifest parent")?
        .parent()
        .context("Devtools plugin root")?;
    for required in [
        "skills/developer-tools/SKILL.md",
        ".mcp.json",
        ".codex/lsp-client.json",
        "lsp/server-catalog.toml",
    ] {
        if !root.join(required).is_file() {
            bail!("Devtools package is missing {required}");
        }
    }
    Ok(Some((manifest_version, marketplace_version)))
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let manifest_path = repo_path(MANIFEST)?;
    if !manifest_path.is_file() {
        return Ok(());
    }
    let marketplace_path = repo_path(MARKETPLACE)?;
    let mut manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&marketplace_path)?;
    manifest["version"] = Value::String(version.to_owned());
    marketplace_plugin_mut_named(&mut marketplace, NAME)?["version"] =
        Value::String(version.to_owned());
    write_json(&manifest_path, &manifest)?;
    write_json(&marketplace_path, &marketplace)
}
