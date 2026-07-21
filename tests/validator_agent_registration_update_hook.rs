use std::path::Path;
use std::process::Command;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_requires_root_bootstrap_and_exact_update_check_hook() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    std::fs::remove_file(plugin_root.join("bootstrap-codexy-agents"))?;
    let missing = validate_roles(&plugin_root)?;
    assert!(!missing.status.success());
    support::assert_structured_literals(
        &stderr(&missing),
        "missing root bootstrap rejection",
        &["plugin root bootstrap"],
    );

    let plugin_root = installed_plugin(&temp.path().join("checker-missing"))?;
    std::fs::remove_file(plugin_root.join("check-codexy-agents"))?;
    let missing_checker = validate_roles(&plugin_root)?;
    assert!(!missing_checker.status.success());
    support::assert_structured_literals(
        &stderr(&missing_checker),
        "missing update checker rejection",
        &["read-only update checker"],
    );

    let plugin_root = installed_plugin(&temp.path().join("second"))?;
    let hook = plugin_root.join("hooks/codexy-routing-context.sh");
    let script = std::fs::read_to_string(&hook)?;
    std::fs::write(
        &hook,
        script.replace("check-codexy-agents\"); then", "check-disabled\"); then"),
    )?;
    let missing_check = validate_roles(&plugin_root)?;
    assert!(!missing_check.status.success());
    support::assert_structured_literals(
        &stderr(&missing_check),
        "missing update check rejection",
        &["exact plugin-root read-only agent update check"],
    );

    let plugin_root = installed_plugin(&temp.path().join("third"))?;
    let hook = plugin_root.join("hooks/codexy-routing-context.sh");
    let script = std::fs::read_to_string(&hook)?;
    std::fs::write(
        &hook,
        script.replace(
            "registration_status=\"UPDATE_REQUIRED\"",
            "registration_status=\"UPDATE_REQUIRED\"\n\"$plugin_root/bootstrap-codexy-agents\"",
        ),
    )?;
    let mutating_call = validate_roles(&plugin_root)?;
    assert!(!mutating_call.status.success());
    support::assert_structured_literals(
        &stderr(&mutating_call),
        "mutating hook bootstrap rejection",
        &["only the exact plugin-root read-only agent update check"],
    );

    let plugin_root = installed_plugin(&temp.path().join("fourth"))?;
    let marker = plugin_root.join("hook-executed");
    std::fs::write(
        plugin_root.join("check-codexy-agents"),
        format!("#!/bin/sh\nprintf ran > '{}'\n", path(&marker)?),
    )?;
    let hook_validation = validate_hooks(&plugin_root)?;
    assert!(!hook_validation.status.success());
    support::assert_structured_literals(
        &stderr(&hook_validation),
        "mutating hook rejected before execution",
        &["checker must match the compiled read-only package"],
    );
    assert!(!marker.exists(), "invalid hook executed before rejection");

    let plugin_root = installed_plugin(&temp.path().join("fifth"))?;
    let hook = plugin_root.join("hooks/codexy-routing-context.sh");
    let script = std::fs::read_to_string(&hook)?;
    std::fs::write(
        &hook,
        script.replace(
            "registration_status=\"UPDATE_REQUIRED\"",
            "registration_status=\"UPDATE_REQUIRED\"\nsuffix=agents\nignored=$(\"$plugin_root/bootstrap-codexy-$suffix\")",
        ),
    )?;
    let computed_call = validate_hooks(&plugin_root)?;
    assert!(!computed_call.status.success());
    support::assert_structured_literals(
        &stderr(&computed_call),
        "computed hook command rejection",
        &["routing hook must match the compiled read-only package"],
    );

    let plugin_root = installed_plugin(&temp.path().join("sixth"))?;
    let marker = plugin_root.join("python-shadow-executed");
    std::fs::write(
        plugin_root.join("argparse.py"),
        format!(
            "from pathlib import Path\nPath({:?}).write_text('ran')\n",
            marker
        ),
    )?;
    let shadow_validation = validate_hooks(&plugin_root)?;
    assert!(shadow_validation.status.success());
    assert!(
        !marker.exists(),
        "Python shadow module was imported by checker"
    );

    let plugin_root = installed_plugin(&temp.path().join("seventh"))?;
    let marker = plugin_root.join("literal-command-executed");
    let hook = plugin_root.join("hooks/codexy-routing-context.sh");
    let mut script = std::fs::read_to_string(&hook)?;
    script.push_str(&format!("\nsh -c 'printf ran > {}'\n", path(&marker)?));
    std::fs::write(&hook, script)?;
    let literal_command = validate_hooks(&plugin_root)?;
    assert!(!literal_command.status.success());
    support::assert_structured_literals(
        &stderr(&literal_command),
        "literal hook command rejected before execution",
        &["compiled read-only package before execution"],
    );
    assert!(
        !marker.exists(),
        "literal hook command executed before rejection"
    );
    Ok(())
}

fn installed_plugin(temp: &Path) -> TestResult<std::path::PathBuf> {
    let plugin_root = temp.join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn validate_roles(plugin_root: &Path) -> TestResult<std::process::Output> {
    validate(plugin_root, "--check-roles")
}

fn validate_hooks(plugin_root: &Path) -> TestResult<std::process::Output> {
    validate(plugin_root, "--check-hooks")
}

fn validate(plugin_root: &Path, mode: &str) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(plugin_root)?, mode])
        .output()?)
}

fn path(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
