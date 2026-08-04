use std::{
    fs,
    path::Path,
    process::Command,
};

use serde_json::{Value, json};

pub(super) fn select_current_bootstrap(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contract_path = repo.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!("1.3.0");
    contract["runtime"]["selectedTag"] = json!("v1.2.2");
    fs::write(&contract_path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/version/bootstrap.rs"),
        repo.join("src/version/bootstrap.rs"),
    )?;
    Ok(())
}

pub(super) fn pre_activation_revision() -> Result<String, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
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
