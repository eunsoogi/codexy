use std::path::Path;
use std::process::Command;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_passive_mandatory_skill_instruction() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/qa/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "- Screenshots are required before handoff.",
        "* Screenshots are required before handoff.",
        "1. Evidence is required.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "addition {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_custom_agent_label_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let agent_path = plugin_root.join("agents/codexy-shipwright.toml");
    let agent = std::fs::read_to_string(&agent_path)?;
    for replacement in [
        (
            "Evidence expectations: MUST provide",
            "Evidence expectations: provide",
        ),
        ("Output format: MUST return", "Output format: return"),
    ] {
        std::fs::write(&agent_path, agent.replace(replacement.0, replacement.1))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_prohibition_list_inversion() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/agents-md-authoring/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replace(
            "MUST NOT remove user-authored policy",
            "MUST remove user-authored policy",
        ),
    )?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_wrapped_prohibition_list_inversion() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/agents-md-authoring/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- MUST NOT rewrite unrelated instructions,\n  MUST remove user-authored policy.\n",
    );
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_wrapped_duplicate_modal_wording() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\nMUST NOT\nMUST treat project agents as installed custom agents.\n");
    skill.push_str("\nMUST use codegraph output to\nMUST identify nearby files.\n");
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("duplicated modal wrapping"));
    Ok(())
}

#[test]
fn validator_cli_allows_real_wrapped_modal_instruction() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\nThe agent MUST use codegraph output to\nidentify nearby files.\n");
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_extra_instruction_after_wrapped_modal_continuation() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for (addition, expected) in [
        (
            "The agent MUST use codegraph output to\nidentify nearby files. Run the validator.",
            "mandatory instructions must use MUST",
        ),
        (
            "The agent MUST use codegraph output to\nidentify nearby files; Run the validator.",
            "mandatory instructions must use MUST",
        ),
        (
            "The agent MUST use codegraph output to\nidentify nearby files; run the validator.",
            "mandatory instructions must use MUST",
        ),
        (
            "The agent MUST use codegraph output to\nidentify nearby files then Run the validator.",
            "mandatory instructions must use MUST",
        ),
        (
            "The agent MUST use codegraph output to\nidentify nearby files. Do not edit files.",
            "prohibitions must use MUST NOT",
        ),
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "addition {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains(expected));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_bare_imperative_after_non_modal_to_from() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "Refer to\nRun the validator.",
        "The `MUST` wording policy refers to\nRun the validator.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "addition {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_markdown_workflow_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/debugging/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "1. Reproduce the smallest failing case.",
        "- Resolve the conflict before handoff.",
        "- List all `.md` files.",
        "- Clone shallowly or use the local repo path.",
        "- Delete `_project.md` after migration.",
        "- [ ] Use WebSearch before handoff.",
        "- [x] Use WebSearch before handoff.",
        "* [ ] Use WebSearch before handoff.",
        "1. [ ] Use WebSearch before handoff.",
        "- [ ] Flag unknown files.",
        "- [ ] Skip generated output.",
        "- [ ] Walk the wiki roots.",
        "- Start a separate Codex thread.",
        "- Give each lane an assignment.",
        "- Complete lane assignment before edits.",
        "- Re-read files before trusting output.",
        "- Add the missing reference.",
        "- Append evidence to the handoff.",
        "- Inspect the current implementation.",
        "- Build the usable experience first.",
        "1. Update the index.",
        "- Choose controls by task.",
        "- Classify each item as proved or missing.",
        "- Extract the helper boundary.",
        "- Move code while preserving contracts.",
        "- Search the callable tool surface.",
        "- Separately record tool search results.",
        "- Mark PASS only with direct evidence.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_text_fence_handoff_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path =
        plugin_root.join("skills/codex-orchestration/references/orchestration-loop.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replace("MUST include goal tool", "Include goal tool"),
    )?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, std::path::PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validator(plugin_root: &Path, mode: &str) -> TestResult<std::process::Output> {
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
