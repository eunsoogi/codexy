use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_remaining_bare_imperative_verbs() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;

    for addition in [
        "- Parse structured files before handoff.",
        "- Name the required Codexy skills.",
        "- Decide whether multi-agent helper work is useful.",
        "- Pull forward the wiki-level findings.",
        "- Open full records only when the user asks for detail.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "instruction {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_modal_wrapped_remaining_imperative_verbs() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- MUST parse structured files before handoff.\n\
         - MUST name the required Codexy skills.\n\
         - MUST decide whether multi-agent helper work is useful.\n\
         - MUST pull forward the wiki-level findings.\n\
         - MUST open full records only when the user asks for detail.\n",
    );
    std::fs::write(&skill_path, skill)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_conditional_clause_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{skill}\n- If verification fails, run the validator.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

#[test]
fn validator_cli_rejects_skill_description_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/task-classification/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    assert!(skill.contains("description: MUST use first"));
    std::fs::write(
        &skill_path,
        skill.replace("description: MUST use first", "description: Use first"),
    )?;

    let output = validator(&plugin_root, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
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
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
