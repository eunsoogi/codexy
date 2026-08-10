use std::path::{Path, PathBuf};

use crate::support;

#[path = "validator_instruction_policy/baseline_contract.rs"]
mod baseline_contract;
#[path = "validator_instruction_policy/loc_exception_exemptions.rs"]
mod loc_exception_exemptions;
#[path = "validator_instruction_policy/loc_exception_policy.rs"]
mod loc_exception_policy;
#[path = "validator_instruction_policy/loc_exception_regressions.rs"]
mod loc_exception_regressions;
#[path = "validator_instruction_policy/loc_exception_sections.rs"]
mod loc_exception_sections;
#[path = "validator_instruction_policy/mandatory_syntax.rs"]
mod mandatory_syntax;
#[path = "validator_instruction_policy/passive_permission.rs"]
mod passive_permission;
#[path = "validator_instruction_policy/sculptor_loc_policy.rs"]
mod sculptor_loc_policy;
#[path = "validator_instruction_policy/repository_skills.rs"]
mod repository_skills;
pub(super) use repository_skills::copy_repo_fixture;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const MUTABLE_PLUGIN_FILES: &[&str] = &[
    ".codex-plugin/plugin.json",
    "agents/codexy-sculptor.toml",
    "agents/codexy-sentinel.toml",
    "agents/codexy-weaver.toml",
    "agents/openai.yaml",
    "skills/orchestration/agents/openai.yaml",
    "skills/git-workflow/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];
const ORCHESTRATION_PROMPT: &str = "You MUST use $orchestration";
#[rustfmt::skip]
const ROOT_AGENTS_BARE_CASES: &[(&str, &str)] = &[("MUST use Codexy codegraph MCP", "Use Codexy codegraph MCP"), ("MUST preflight branch refs", "preflight branch refs"), ("MUST wait", "Wait"), ("MUST keep metadata current", "Keep metadata current"), ("MUST add nested", "Add nested"), ("MUST put executable", "Put executable"), ("MUST treat failures", "Treat failures"), ("MUST capture", "Capture"), ("MUST mention unrelated", "Mention unrelated")];

#[test]
fn validator_cli_rejects_agent_instruction_policy_false_negative() -> TestResult {
    let fixture = support::instruction_policy_fixture(Path::new("agents/codexy-sentinel.toml"))?;
    let agent_path = fixture.path();
    let agent = std::fs::read_to_string(&agent_path)?;
    for replacement in ["shall not edit files", "do not edit files"] {
        std::fs::write(
            &agent_path,
            agent.replace("MUST NOT edit files", replacement),
        )?;
        let output = support::validator_instruction_policy_file(agent_path)?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("MUST NOT"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_bare_run_instruction() -> TestResult {
    let fixture =
        support::instruction_policy_fixture(Path::new("skills/proof-driven-completion/SKILL.md"))?;
    let skill_path = fixture.path();
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
        let output = support::validator_instruction_policy_file(skill_path)?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_allows_tilde_fenced_command_examples() -> TestResult {
    let fixture =
        support::instruction_policy_fixture(Path::new("skills/proof-driven-completion/SKILL.md"))?;
    let skill_path = fixture.path();
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n~~~sh\nRun dangerous-example\n~~~\n> No wiki found. Run `/wiki init` first.\n",
    );
    std::fs::write(&skill_path, skill)?;
    let output = support::validator_instruction_policy_file(skill_path)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_root_agents_policy_false_negative() -> TestResult {
    let (_temp, plugin_root, agents_path) = repository_skills::copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    support::assert_structured_literals(
        &agents,
        "root AGENTS prohibition policy",
        &["MUST NOT` for prohibitions"],
    );
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
    let (_temp, plugin_root, agents_path) = repository_skills::copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    for (required, bare) in ROOT_AGENTS_BARE_CASES {
        support::assert_structured_literals(&agents, "root AGENTS mandatory policy", &[required]);
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
        "Run $orchestration before setup, then create a branch.",
        "You MUST run $orchestration, then use $orchestration.",
        "You MUST track goals; assign specialist roles.",
        "You MUST verify evidence, and use squash-merge gates.",
        "Stop and fix if proof contradicts the claim.",
        "Maintain a visible todo list.",
        "You MUST use skills, keep routing hidden.",
        "Re-run $orchestration before setup.",
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
    support::assert_structured_literals(
        &original,
        "agent default prompt policy",
        &[ORCHESTRATION_PROMPT],
    );
    for prompt in [
        "Run $orchestration before setup, then create a branch.",
        "You MUST run $orchestration, then use $orchestration.",
        "You MUST track goals; assign specialist roles.",
        "You MUST verify evidence, and use squash-merge gates.",
        "Stop and fix if proof contradicts the claim.",
        "Maintain a visible todo list.",
        "You MUST use skills, keep routing hidden.",
        "Re-run $orchestration",
        "Drive external surfaces directly.",
        "Track goals and todos.",
        "Check priority before writing.",
        "Keep real todo/plan state current.",
    ] {
        std::fs::write(
            &prompt_path,
            {
                let mutated = original.replace(ORCHESTRATION_PROMPT, prompt);
                assert_ne!(original, mutated, "fixture prompt was not mutated");
                mutated
            },
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
    let skill_prompt = plugin_root.join("skills/orchestration/agents/openai.yaml");
    let skill = std::fs::read_to_string(&skill_prompt)?;
    std::fs::write(
        &skill_prompt,
        skill.replace("MUST report the limitation", "report the limitation"),
    )?;
    assert!(!validator(&plugin_root, "--check")?.status.success());
    Ok(())
}

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let mutable_files = MUTABLE_PLUGIN_FILES
        .iter()
        .map(Path::new)
        .collect::<Vec<_>>();
    Ok(support::copy_plugin_fixture_with_mutable_files(
        &mutable_files,
    )?)
}

fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    if mode == "--check" {
        return support::validator_instruction_policy(plugin_root);
    }
    support::validator(plugin_root, mode)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
