use std::path::Path;
use std::process::{Command, Output};

mod support;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "validator_sentinel_reviewer_gate/approval_boundaries.rs"]
mod approval_boundaries;
#[path = "validator_sentinel_reviewer_gate/baseline_contract.rs"]
mod baseline_contract;
#[path = "validator_sentinel_reviewer_gate/evidence_regressions.rs"]
mod evidence_regressions;

fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|sentinel| sentinel.replace(needle, replacement))
}

fn validate_sentinel_edit(edit: impl FnOnce(String) -> String) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel))?;
    validator(&plugin_root)
}

fn copy_fixture(plugin_root: &Path) -> std::io::Result<()> {
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
