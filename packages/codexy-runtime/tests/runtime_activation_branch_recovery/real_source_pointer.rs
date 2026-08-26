use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;

#[test]
fn current_source_checkout_keeps_the_base_public_bootstrap_control() -> Result<(), Box<dyn std::error::Error>> {
    assert_base_source_pointer(codexy_runtime::paths::repository_root())
}

pub(super) fn assert_base_source_pointer(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let release: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    assert_eq!(release["state"], "legacy-public");
    assert_eq!(release["artifact"]["tag"], "v1.2.2");
    assert_eq!(
        release["source"]["commit"],
        "6890b3089dcffc2293f8f63b761e33562250eac6"
    );
    assert_eq!(
        release["platforms"].as_object().map(|items| items.len()),
        Some(2)
    );
    let wrapper = fs::read_to_string(root.join(
        "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
    ))?;
    assert!(wrapper.contains("exec uvx --from getcodexy==1.2.2"));
    assert!(!wrapper.contains("getcodexy==1.5.0"));
    Ok(())
}

pub(super) fn assert_activated_source_pointer(
    root: &Path,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let release: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    assert_eq!(release["state"], "source-selected");
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
