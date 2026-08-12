use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{
    MARKETPLACE, fields, load_json, marketplace_plugin_mut_named, repo_path,
    require_matching_version, string_field, write_json,
};

const NAME: &str = "codexy-github";
const MANIFEST: &str = "plugins/codexy-github/.codex-plugin/plugin.json";
const SOURCE: &str = "./plugins/codexy-github";

pub(super) fn check(core_version: &str) -> Result<()> {
    let manifest_path = repo_path(MANIFEST)?;
    let marketplace_path = repo_path(MARKETPLACE)?;
    let manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&marketplace_path)?;
    if string_field(&manifest, "name", "GitHub plugin manifest")? != NAME {
        bail!("GitHub plugin manifest name must be {NAME:?}");
    }
    require_matching_version(
        string_field(&manifest, "version", "GitHub plugin manifest")?,
        MANIFEST,
        core_version,
        "core plugin manifest",
    )?;
    if string_field(&manifest, "skills", "GitHub plugin manifest")? != "./skills/" {
        bail!("GitHub plugin manifest skills must be ./skills/");
    }
    let entry = marketplace_plugin_mut_named(&mut marketplace, NAME)?;
    require_matching_version(
        string_field(entry, "version", "GitHub marketplace entry")?,
        MARKETPLACE,
        core_version,
        "core plugin manifest",
    )?;
    if entry.pointer("/source/path").and_then(Value::as_str) != Some(SOURCE) {
        bail!("GitHub marketplace source must be {SOURCE:?}");
    }
    if fields::string_array(&manifest, "supportedPlatforms", "GitHub plugin manifest")?
        != fields::string_array(entry, "supportedPlatforms", "GitHub marketplace entry")?
    {
        bail!("GitHub plugin supportedPlatforms must match its marketplace entry");
    }
    if !manifest_path
        .parent()
        .context("GitHub manifest parent")?
        .parent()
        .context("GitHub plugin root")?
        .join("skills/git-workflow/SKILL.md")
        .is_file()
    {
        bail!("GitHub plugin skill is missing");
    }
    Ok(())
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let manifest_path = repo_path(MANIFEST)?;
    let marketplace_path = repo_path(MARKETPLACE)?;
    let mut manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&marketplace_path)?;
    manifest["version"] = Value::String(version.to_owned());
    marketplace_plugin_mut_named(&mut marketplace, NAME)?["version"] =
        Value::String(version.to_owned());
    write_json(&manifest_path, &manifest)?;
    write_json(&marketplace_path, &marketplace)
}
