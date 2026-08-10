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
    let plugin = repo.join("plugins/codexy");
    let mut manifest: Value = serde_json::from_slice(&fs::read(
        plugin.join(".codex-plugin/plugin.json"),
    )?)?;
    fs::remove_dir_all(&plugin)?;
    copy_dir(root.join("plugins/codexy"), &plugin)?;
    let candidate: Value = serde_json::from_slice(&fs::read(
        plugin.join(".codex-plugin/plugin.json"),
    )?)?;
    manifest["interface"]["defaultPrompt"] = candidate["interface"]["defaultPrompt"].clone();
    fs::write(
        plugin.join(".codex-plugin/plugin.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    for relative in [
        "docs/getcodexy-component-installation.md",
        "packages/getcodexy/contracts/component-installation-contract.json",
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
    ] {
        let target = repo.join(relative);
        fs::create_dir_all(target.parent().ok_or("component contract parent")?)?;
        fs::copy(root.join(relative), target)?;
    }
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
