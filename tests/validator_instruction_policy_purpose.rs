use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_modal_purpose_clauses_with_prohibition_words() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- Evidence handoffs MUST include exact heads so future agents cannot confuse stale review output with current proof.\n",
    );
    skill.push_str("- When the check cannot run, MUST stop and report the blocker.\n");
    skill.push_str("- Review summaries MUST stop when the check cannot run.\n");
    skill.push_str("- Review summaries MUST name exact scope to avoid stale handoff claims.\n");
    std::fs::write(&skill_path, skill)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_rejects_true_prohibitions_without_must_not() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "- Evidence handoffs cannot omit exact heads.\n",
        "- Avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope, but cannot omit current proof.\n",
        "- Review summaries MUST include exact scope and avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope when available, but cannot omit current proof.\n",
        "- Review summaries MUST name exact scope to help reviewers and avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope when available. Cannot omit current proof.\n",
        "- Review summaries MUST include exact scope when available, cannot omit current proof.\n",
        "- Review summaries MUST include exact scope when available then cannot omit current proof.\n",
        "- Review summaries MUST report that stale output cannot prove current state, but avoid claiming stale output is current proof.\n",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "addition {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    }
    Ok(())
}

#[test]
fn validator_allows_separate_must_action_after_prohibition() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/agents-md-authoring/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- MUST NOT leave temp servers running, and MUST add the cleanup receipt to the handoff.\n",
    );
    std::fs::write(&skill_path, skill)?;
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
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
