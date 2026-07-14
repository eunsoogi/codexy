use std::path::{Path, PathBuf};
use std::process::Command;

mod support;

#[path = "validator_instruction_policy/baseline_contract.rs"]
mod baseline_contract;
#[path = "validator_instruction_policy/loc_exception_exemptions.rs"]
mod loc_exception_exemptions;
#[path = "validator_instruction_policy/loc_exception_policy.rs"]
mod loc_exception_policy;
#[path = "validator_instruction_policy/loc_exception_sections.rs"]
mod loc_exception_sections;
#[path = "validator_instruction_policy/sculptor_loc_policy.rs"]
mod sculptor_loc_policy;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[rustfmt::skip]
const ROOT_AGENTS_BARE_CASES: &[(&str, &str)] = &[("MUST use Codexy codegraph MCP", "Use Codexy codegraph MCP"), ("MUST preflight branch refs", "preflight branch refs"), ("MUST wait", "Wait"), ("MUST keep metadata current", "Keep metadata current"), ("MUST add nested", "Add nested"), ("MUST put executable", "Put executable"), ("MUST treat failures", "Treat failures"), ("MUST capture", "Capture"), ("MUST mention unrelated", "Mention unrelated")];

#[test]
fn validator_cli_rejects_agent_instruction_policy_false_negative() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
    let agent = std::fs::read_to_string(&agent_path)?;
    for replacement in ["shall not edit files", "do not edit files"] {
        std::fs::write(
            &agent_path,
            agent.replace("MUST NOT edit files", replacement),
        )?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("MUST NOT"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_same_line_prohibition_with_must_not_elsewhere() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- Do not edit files; this line mentions MUST NOT as policy text.\n");
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_lowercase_must_and_must_not() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- must run `git diff --check`.\n- must not edit files.\n");
    std::fs::write(&skill_path, skill)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("mandatory instructions must use MUST"));
    assert!(stderr.contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_bare_run_instruction() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "- MUST run `git diff --check`. Use Codexy codegraph MCP. Stop parent implementation routing. Stage only intended files. Preserve unrelated dirty work.",
        "> **Policy.** MUST NOT continue. Stop parent implementation routing.",
        "- Stage only intended files.",
        "- Preserve unrelated dirty work.",
        "- Follow this protocol exactly.",
        "1. **Read `~/.config/example.json`**.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_allows_tilde_fenced_command_examples() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n~~~sh\nRun dangerous-example\n~~~\n> No wiki found. Run `/wiki init` first.\n",
    );
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_root_agents_policy_false_negative() -> TestResult {
    let (_temp, plugin_root, agents_path) = copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    assert!(agents.contains("MUST NOT` for prohibitions"));
    std::fs::write(
        &agents_path,
        agents.replace("MUST NOT` for prohibitions", "do not` for prohibitions"),
    )?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_root_agents_bare_use_instruction() -> TestResult {
    let (_temp, plugin_root, agents_path) = copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    for (required, bare) in ROOT_AGENTS_BARE_CASES {
        assert!(agents.contains(required));
        std::fs::write(&agents_path, agents.replace(required, bare))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_manifest_default_prompt_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
    for prompt in [
        "Run $task-classification before setup, then create a branch.",
        "You MUST run $task-classification, then use $codex-orchestration.",
        "You MUST track goals; assign specialist roles.",
        "You MUST verify evidence, and use squash-merge gates.",
        "Stop and fix if proof contradicts the claim.",
        "Maintain a visible todo list.",
        "You MUST use skills, keep routing hidden.",
        "Re-run $task-classification before setup.",
        "Drive external surfaces directly.",
        "Track goals and todos.",
        "Check priority before writing.",
        "Keep real todo/plan state current.",
    ] {
        manifest["interface"]["defaultPrompt"][0] = serde_json::json!(prompt);
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "prompt {prompt:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_yaml_default_prompt_bare_imperatives() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let original = std::fs::read_to_string(&prompt_path)?;
    assert!(original.contains("You MUST run $task-classification"));
    for prompt in [
        "Run $task-classification before setup, then create a branch.",
        "You MUST run $task-classification, then use $codex-orchestration.",
        "You MUST track goals; assign specialist roles.",
        "You MUST verify evidence, and use squash-merge gates.",
        "Stop and fix if proof contradicts the claim.",
        "Maintain a visible todo list.",
        "You MUST use skills, keep routing hidden.",
        "Re-run $task-classification",
        "Drive external surfaces directly.",
        "Track goals and todos.",
        "Check priority before writing.",
        "Keep real todo/plan state current.",
    ] {
        std::fs::write(
            &prompt_path,
            original.replace("You MUST run $task-classification", prompt),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "prompt {prompt:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    std::fs::write(
        &prompt_path,
        format!("{original}\nguidance: \"Run child setup.\"\n"),
    )?;
    assert!(!validator(&plugin_root, "--check")?.status.success());
    let skill_prompt = plugin_root.join("skills/codex-orchestration/agents/openai.yaml");
    let skill = std::fs::read_to_string(&skill_prompt)?;
    std::fs::write(
        &skill_prompt,
        skill.replace("MUST report the limitation", "report the limitation"),
    )?;
    assert!(!validator(&plugin_root, "--check")?.status.success());
    Ok(())
}

fn copy_fixture(plugin_root: &Path) -> std::io::Result<()> {
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    Ok((temp, plugin_root))
}

fn copy_repo_fixture() -> TestResult<(tempfile::TempDir, PathBuf, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let repo_root = temp.path().join("repo");
    let plugin_root = repo_root.join("plugins/codexy");
    let agents_path = repo_root.join("AGENTS.md");
    std::fs::create_dir_all(repo_root.join("plugins"))?;
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"),
        &agents_path,
    )?;
    copy_fixture(&plugin_root)?;
    Ok((temp, plugin_root, agents_path))
}

fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
