use std::{
    fs,
    path::Path,
    process::Command,
};

use serde_json::{Value, json};

use crate::support::copy_dir;

pub(super) fn synchronize_current_plugin_validation_inputs(
    repo: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    fs::copy(root.join(".gitattributes"), repo.join(".gitattributes"))?;
    let core_plugin = repo.join("plugins/codexy");
    let plugin = repo.join("plugins/codexy-devtools");
    let mut manifest: Value = serde_json::from_slice(&fs::read(
        core_plugin.join(".codex-plugin/plugin.json"),
    )?)?;
    let baseline_version = manifest["version"]
        .as_str()
        .ok_or("fixture core manifest version")?
        .to_owned();
    if plugin.exists() {
        fs::remove_dir_all(&plugin)?;
    }
    copy_dir(root.join("plugins/codexy-devtools"), &plugin)?;
    let core_candidate: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    manifest["interface"]["defaultPrompt"] = core_candidate["interface"]["defaultPrompt"].clone();
    fs::write(core_plugin.join(".codex-plugin/plugin.json"), format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    copy_dir(root.join("plugins/codexy-github"), &repo.join("plugins/codexy-github"))?;
    for relative in [
        ".agents/plugins/marketplace.json",
        "docs/getcodexy-component-installation.md",
        "packages/getcodexy/contracts/component-installation-contract.json",
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
    ] {
        let target = repo.join(relative);
        fs::create_dir_all(target.parent().ok_or("component contract parent")?)?;
        fs::copy(root.join(relative), target)?;
    }
    reset_github_version(repo, &baseline_version)?;
    Ok(())
}

fn reset_github_version(repo: &Path, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = repo.join("plugins/codexy-github/.codex-plugin/plugin.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["version"] = Value::String(version.to_owned());
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;

    let marketplace_path = repo.join(".agents/plugins/marketplace.json");
    let mut marketplace: Value = serde_json::from_slice(&fs::read(&marketplace_path)?)?;
    let plugin = marketplace["plugins"]
        .as_array_mut()
        .and_then(|plugins| plugins.iter_mut().find(|item| item["name"] == "codexy-github"))
        .ok_or("fixture GitHub marketplace entry")?;
    plugin["version"] = Value::String(version.to_owned());
    fs::write(&marketplace_path, format!("{}\n", serde_json::to_string_pretty(&marketplace)?))?;
    Ok(())
}

pub(super) fn assert_canonical_default_prompt(
    repo: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(
        repo.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let prompt = manifest["interface"]["defaultPrompt"]
        .as_array()
        .ok_or("candidate manifest defaultPrompt")?;
    if !prompt.iter().any(|item| {
        item.as_str()
            .is_some_and(|text| text.contains("$orchestration"))
    }) {
        return Err("candidate manifest defaultPrompt omits $orchestration".into());
    }
    for retired in [
        "$task-classification",
        "$codex-orchestration",
        "$token-efficient-orchestration",
    ] {
        if prompt
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text.contains(retired)))
        {
            return Err(format!("candidate manifest retains retired route {retired}").into());
        }
    }
    Ok(())
}

pub(super) fn assert_canonical_wrapper_eol(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for wrapper in ["plugins/codexy-devtools/mcp/codexy-mcp-lsp", "plugins/codexy-devtools/mcp/codexy-mcp-codegraph"] {
        let output = Command::new("git")
            .args(["check-attr", "text", "eol", "--", wrapper])
            .current_dir(repo)
            .output()?;
        let expected = format!("{wrapper}: text: set\n{wrapper}: eol: lf\n");
        if !output.status.success() || output.stdout != expected.as_bytes() {
            return Err(format!("fixture wrapper EOL contract mismatch: {wrapper}").into());
        }
    }
    Ok(())
}

pub(super) fn select_current_bootstrap(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contract_path = repo.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!("1.3.0");
    contract["runtime"]["selectedTag"] = json!("v1.2.2");
    fs::write(&contract_path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    fs::copy(
        codexy_runtime::paths::runtime_package_root().join("src/version/bootstrap.rs"),
        repo.join("packages/codexy-runtime/src/version/bootstrap.rs"),
    )?;
    Ok(())
}

pub(super) fn pre_activation_revision() -> Result<String, Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let current: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let selected = current["bootstrap"]["selectedVersion"]
        .as_str()
        .ok_or("current selected bootstrap version")?;
    let candidate = current["bootstrap"]["candidateVersion"]
        .as_str()
        .ok_or("current candidate bootstrap version")?;
    let revisions = Command::new("git")
        .args([
            "log",
            "--format=%H",
            "--",
            ".agents/plugins/release-publish-contract.json",
        ])
        .current_dir(root)
        .output()?;
    if !revisions.status.success() {
        return Err("unable to list release contract history".into());
    }
    for revision in String::from_utf8(revisions.stdout)?.lines() {
        let source = Command::new("git")
            .arg("show")
            .arg(format!("{revision}:.agents/plugins/release-publish-contract.json"))
            .current_dir(root)
            .output()?;
        if !source.status.success() {
            continue;
        }
        let contract: Value = serde_json::from_slice(&source.stdout)?;
        if contract["bootstrap"]["candidateVersion"] == candidate
            && contract["bootstrap"]["selectedVersion"] != selected
        {
            return Ok(revision.to_owned());
        }
    }
    Err("release contract history has no pre-activation candidate baseline".into())
}
