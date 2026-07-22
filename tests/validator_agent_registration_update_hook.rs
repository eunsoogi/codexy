use std::path::Path;
use std::process::Command;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn role_validator_requires_root_bootstrap_and_exact_update_checker() -> TestResult {
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
