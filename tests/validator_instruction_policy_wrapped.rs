use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_wrapped_modal_continuation_prohibition() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{skill}\nThe agent MUST use codegraph output to\navoid direct edits.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_accepts_wrapped_modal_continuation_without_new_instruction() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{skill}\nThe agent MUST use codegraph output to\nidentify nearby files.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validator(plugin_root: &Path, mode: &str) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            mode,
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
