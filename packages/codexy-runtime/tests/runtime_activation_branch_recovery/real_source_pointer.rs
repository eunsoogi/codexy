use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;

#[test]
fn current_source_checkout_exposes_the_selected_runtime_pointer() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let selected_version = selected_runtime_version(&root)?;
    assert_activated_source_pointer(&root, &selected_version)
}

fn selected_runtime_version(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let contract: Value = serde_json::from_slice(&fs::read(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let tag = contract["runtime"]["selectedTag"]
        .as_str()
        .ok_or("selected runtime tag must be a string")?;
    Ok(tag
        .strip_prefix('v')
        .ok_or("selected runtime tag must start with v")?
        .to_owned())
}

pub(super) fn assert_activated_source_pointer(
    root: &Path,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let release: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    let selected_version = selected_runtime_version(root)?;
    assert_eq!(release["state"], "source-selected");
    assert_eq!(selected_version, expected_version);
    assert_eq!(release["artifact"]["tag"], format!("v{expected_version}"));
    assert_eq!(release["source"]["repository"], "https://github.com/eunsoogi/codexy");
    assert!(release["source"]["commit"].as_str().is_some_and(|value| value.len() == 40));
    assert_eq!(
        release["platforms"].as_object().map(|items| items.len()),
        Some(2)
    );
    assert!(!root.join("plugins/codexy-devtools/runtime-candidate.json").exists());
    let wrapper = fs::read_to_string(root.join(
        "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
    ))?;
    assert!(wrapper.contains(&format!("getcodexy=={expected_version}")));
    Ok(())
}

pub(super) fn assert_result(output: Output, success: bool, case: &str) {
    assert_eq!(
        output.status.success(),
        success,
        "unexpected {case} result\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

pub(super) fn restore_pre_activation_runtime_inputs(
    repo: &Path,
    revision: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for relative in [
        "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
        "plugins/codexy-devtools/runtime-release.json",
    ] {
        let output = Command::new("git")
            .args(["show", &format!("{revision}:{relative}")])
            .current_dir(codexy_runtime::paths::repository_root())
            .output()?;
        if !output.status.success() {
            return Err(format!("git show failed for {relative}").into());
        }
        fs::write(repo.join(relative), output.stdout)?;
    }
    Ok(())
}
