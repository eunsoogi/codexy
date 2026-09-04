use std::process::Command;

#[allow(unused)]
use crate::support;

#[path = "validator_hooks/admission_artifact.rs"]
mod admission_artifact;
#[path = "validator_hooks/admission_runtime.rs"]
mod admission_runtime;
#[path = "validator_hooks/capability_contract.rs"]
mod capability_contract;
#[path = "validator_hooks/merge_admission.rs"]
mod merge_admission;
#[cfg(unix)]
#[path = "validator_hooks/merge_admission_line_endings.rs"]
mod merge_admission_line_endings;
#[path = "validator_hooks/github_authorization_pagination.rs"]
mod github_authorization_pagination;
#[path = "validator_hooks/filesystem_aliases.rs"]
mod filesystem_aliases;
#[path = "validator_hooks/graphql_admission.rs"]
mod graphql_admission;
#[path = "validator_hooks/shell_context_regressions.rs"]
mod shell_context_regressions;
#[path = "validator_hooks/thread_delivery_parent_route.rs"]
mod thread_delivery_parent_route;
#[path = "validator_hooks/thread_delivery_support.rs"]
mod thread_delivery_support;
#[path = "validator_hooks/thread_delivery_diagnostics.rs"]
mod thread_delivery_diagnostics;
#[path = "validator_hooks/thread_delivery_missing_fields.rs"]
mod thread_delivery_missing_fields;
#[path = "validator_hooks/repository_github_policy_config.rs"]
mod repository_github_policy_config;
#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;

#[test]
fn policy_inventory_boundary_is_removed() -> Result<(), Box<dyn std::error::Error>> {
    let repository = codexy_runtime::paths::repository_root();
    for path in [
        "plugins/codexy/hooks/policy-inventory.json",
        "scripts/generate-hook-policy-inventory",
        "scripts/policy_inventory_review_decisions.py",
        "packages/codexy-runtime/src/validation/hooks/policy_inventory.rs",
        "packages/codexy-runtime/src/validation/hooks/policy_inventory_contract.rs",
        "packages/codexy-runtime/src/validation/hooks/policy_inventory_discovery.rs",
        "packages/codexy-runtime/src/validation/hooks/policy_inventory_frontmatter.rs",
        "packages/codexy-runtime/src/validation/hooks/policy_inventory_suite.rs",
    ] {
        assert!(
            !repository.join(path).exists(),
            "removed policy inventory boundary remains at {path}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_missing_hooks_configuration() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::remove_file(root.join("hooks/hooks.json"))?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(text(&output).contains("hooks/hooks.json"));
    Ok(())
}

#[test]
fn validator_rejects_unsafe_generic_commands() -> Result<(), Box<dyn std::error::Error>> {
    for command in [
        "./hooks/codexy-issue-title-check.sh --issue-title Valid",
        "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\"; touch /tmp/pwned",
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        set_command(&root, command)?;
        assert!(
            !validate(&root)?.status.success(),
            "validator accepted {command}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_non_boolean_generic_hook_async() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    set_command(
        &root,
        "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" --issue-title Valid",
    )?;
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"][0]["hooks"][0]["async"] = serde_json::json!("false");
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(text(&output).contains("hook async must be a boolean"));
    Ok(())
}

#[test]
fn validator_does_not_apply_github_topology_to_an_unrelated_extension()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("unrelated");
    std::fs::create_dir_all(root.join(".codex-plugin"))?;
    std::fs::create_dir_all(root.join("hooks"))?;
    std::fs::write(root.join(".codex-plugin/plugin.json"), r#"{"name":"unrelated","version":"1.0.0"}"#)?;
    std::fs::write(root.join("hooks/hooks.json"), r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"\"${PLUGIN_ROOT}/hooks/context.sh\"","commandWindows":"\"${PLUGIN_ROOT}/hooks/context.cmd\"","timeout":1}]}]}}"#)?;
    std::fs::write(root.join("hooks/context.sh"), "#!/bin/sh\nexit 0\n")?;
    std::fs::write(root.join("hooks/context.cmd"), "@echo off\nexit /b 0\n")?;
    let output = validate(&root)?;
    assert!(output.status.success(), "{}", text(&output));
    Ok(())
}

pub(super) fn copy(base: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = base.join("codexy");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &root,
    )?;
    let admission_suite = base.join("packages/codexy-runtime/tests/suites/all.rs");
    std::fs::create_dir_all(admission_suite.parent().ok_or("admission suite parent")?)?;
    std::fs::write(admission_suite, "// admission runtime suite\n")?;
    Ok(root)
}
pub(super) fn copy_github(base: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = base.join("codexy-github");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy-github"),
        &root,
    )?;
    let admission_suite = base.join("packages/codexy-runtime/tests/suites/all.rs");
    std::fs::create_dir_all(admission_suite.parent().ok_or("admission suite parent")?)?;
    std::fs::write(admission_suite, "// admission runtime suite\n")?;
    Ok(root)
}
pub(super) fn set_command(root: &std::path::Path, command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"] =
        serde_json::json!([{"hooks":[{"type":"command","command":command,"timeout":3}]}]);
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(())
}
pub(super) fn read(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
pub(super) fn validate(root: &std::path::Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("root")?,
            "--check-hooks",
        ])
        .output()?)
}
pub(super) fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
