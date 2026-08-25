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
    let devtools_manifest_path = plugin.join(".codex-plugin/plugin.json");
    let mut devtools_manifest: Value = serde_json::from_slice(&fs::read(&devtools_manifest_path)?)?;
    devtools_manifest["version"] = Value::String(baseline_version.clone());
    fs::write(
        &devtools_manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&devtools_manifest)?),
    )?;
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
        "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
        "packages/getcodexy/uv.lock",
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
        "scripts/download-selected-runtime-package.sh",
    ] {
        let target = repo.join(relative);
        fs::create_dir_all(target.parent().ok_or("component contract parent")?)?;
        fs::copy(root.join(relative), target)?;
    }
    let current_contract: Value = serde_json::from_slice(&fs::read(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let contract_path = repo.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path)?)?;
    contract["bootstrap"]["candidateVersion"] =
        current_contract["bootstrap"]["candidateVersion"].clone();
    fs::write(&contract_path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    reset_github_version(repo, &baseline_version)?;
    Ok(())
}

pub(super) fn make_uv_lock_stale(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let lock = repo.join("packages/getcodexy/uv.lock");
    let lock_text = fs::read_to_string(&lock)?;
    let current_version = lock_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        .ok_or("getcodexy lock version")?;
    let stale_version = next_patch_version(current_version)?;
    let stale_lock = lock_text.replacen(
        &format!("version = \"{current_version}\""),
        &format!("version = \"{stale_version}\""),
        1,
    );
    fs::write(lock, stale_lock)?;
    Ok(())
}

fn next_patch_version(version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (major, remainder) = version.split_once('.').ok_or("major version")?;
    let (minor, patch) = remainder.split_once('.').ok_or("minor version")?;
    let patch = patch.parse::<u64>()?.checked_add(1).ok_or("patch overflow")?;
    Ok(format!("{major}.{minor}.{patch}"))
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

pub(super) fn assert_canonical_preserved_eol(
    repo: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for path in [
        "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
        "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
        "plugins/codexy-devtools/runtime-release.json",
    ] {
        let output = Command::new("git")
            .args(["check-attr", "text", "eol", "--", path])
            .current_dir(repo)
            .output()?;
        let expected = format!("{path}: text: set\n{path}: eol: lf\n");
        if !output.status.success() || output.stdout != expected.as_bytes() {
            return Err(format!("fixture preserved EOL contract mismatch: {path}").into());
        }
    }
    Ok(())
}

pub(super) fn enable_autocrlf(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for args in [
        &["config", "core.autocrlf", "true"][..],
        &["reset", "--hard", "HEAD"][..],
    ] {
        let status = Command::new("git").args(args).current_dir(repo).status()?;
        if !status.success() {
            return Err("unable to configure fixture autocrlf".into());
        }
    }
    Ok(())
}

pub(super) fn select_current_bootstrap(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let selected_version = current_candidate_version()?;
    let runtime_release: Value = serde_json::from_slice(&fs::read(
        repo.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    let selected_tag = runtime_release["artifact"]["tag"]
        .as_str()
        .ok_or("selected runtime release tag")?;
    let contract_path = repo.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!(selected_version);
    contract["bootstrap"]["candidateVersion"] = json!(selected_version);
    contract["runtime"]["selectedTag"] = json!(selected_tag);
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    assert_eq!(contract["bootstrap"]["candidateVersion"], selected_version);
    assert_eq!(contract["runtime"]["selectedTag"], selected_tag);
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
        if is_pre_activation_baseline(&contract, selected) {
            return Ok(revision.to_owned());
        }
    }
    Err("release contract history has no pre-activation candidate baseline".into())
}

pub(super) fn current_candidate_version() -> Result<String, Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    contract["bootstrap"]["candidateVersion"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "current candidate bootstrap version".into())
}

fn is_pre_activation_baseline(contract: &Value, selected: &str) -> bool {
    contract["bootstrap"]["candidateVersion"] == selected
        && contract["bootstrap"]["selectedVersion"] != selected
}
