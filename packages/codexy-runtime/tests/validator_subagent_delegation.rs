use std::path::Path;
use std::process::{Command, Output};

#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;
use crate::support::{self, PluginFixture};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NO_RECURSIVE_DELEGATION: &str = "MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.";

#[test]
fn validator_accepts_all_packaged_roles_with_nonrecursive_delegation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    for role in [
        "codexy-architect",
        "codexy-auditor",
        "codexy-cartographer",
        "codexy-forge",
        "codexy-pathfinder",
        "codexy-scribe",
        "codexy-sculptor",
        "codexy-sentinel",
        "codexy-shipwright",
        "codexy-tracer",
        "codexy-warden",
        "codexy-weaver",
    ] {
        let role =
            std::fs::read_to_string(root.join(format!("plugins/codexy/agents/{role}.toml")))?;
        structured_contract::assert_rules(
            &structured_contract::Contract::markdown_for_subject(&role, "agent"),
            &structured_contract_rules::DELEGATION[..1],
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_role_without_nonrecursive_delegation_prohibition() -> TestResult {
    let fixture = plugin_fixture()?;
    let role_path = fixture.root().join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replace(NO_RECURSIVE_DELEGATION, "MUST NOT recurse through helpers."),
    )?;

    let output = validator_cli(fixture.root())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("nonrecursive delegation contract is missing"));
    Ok(())
}

#[test]
fn validator_rejects_role_that_permits_recursive_delegation() -> TestResult {
    let fixture = plugin_fixture()?;
    for permission in [
        "MAY spawn another helper after mapping the repository.",
        "CAN spawn a helper after mapping the repository.",
        "MAY delegate work to a helper after mapping the repository.",
        "Allowed actions: spawn helper tasks after mapping the repository.",
        "A helper is allowed to spawn another helper after mapping the repository.",
        "Allowed actions: spawn helper tasks, but MUST NOT merge.",
        "Permitted to delegate to a reviewer thread after mapping the repository.",
    ] {
        let role_path = reset_fixture_file(&fixture, Path::new("agents/codexy-cartographer.toml"))?;
        let role = std::fs::read_to_string(&role_path)?;
        std::fs::write(
            &role_path,
            role.replacen("\n\"\"\"", &format!("\n{permission}\n\"\"\""), 1),
        )?;

        let output = validator(fixture.root())?;

        assert!(!output.status.success(), "{permission}");
        assert!(
            stderr(&output).contains("permits recursive delegation"),
            "{permission}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_orchestration_without_first_level_delegation_contract() -> TestResult {
    let fixture = plugin_fixture()?;
    let skill_path = fixture.root().join("skills/orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replace(
            "A child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers.",
            "A child implementation thread MAY not spawn first-level helpers.",
        ),
    )?;

    let output = validator(fixture.root())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("nonrecursive delegation contract is missing"));
    Ok(())
}

#[test]
fn validator_rejects_recursive_permission_in_orchestration_skill() -> TestResult {
    let fixture = plugin_fixture()?;
    let path = fixture.root().join("skills/orchestration/SKILL.md");
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        &path,
        text.replace(
            NO_RECURSIVE_DELEGATION,
            "MAY create an additional reviewer task.",
        ),
    )?;

    let output = validator(fixture.root())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("permits recursive delegation"));
    Ok(())
}

#[test]
fn validator_rejects_recursive_permission_in_every_registered_reference() -> TestResult {
    let fixture = plugin_fixture()?;
    for relative_path in registered_orchestration_references()? {
        let path = reset_fixture_file(&fixture, Path::new(&relative_path))?;
        let mut text = std::fs::read_to_string(&path)?;
        text.push_str("\nA helper MAY spawn another helper.\n");
        std::fs::write(path, text)?;

        let output = validator(fixture.root())?;

        assert!(!output.status.success(), "{relative_path}");
        assert!(
            stderr(&output).contains("permits recursive delegation"),
            "{relative_path}"
        );
    }
    Ok(())
}

#[test]
fn packaged_contract_allows_child_helpers_and_forbids_helper_recursion() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/orchestration/SKILL.md"))?;
    let loop_reference = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/orchestration-loop.md"),
    )?;

    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&orchestration),
        &structured_contract_rules::DELEGATION[1..2],
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&loop_reference),
        &structured_contract_rules::DELEGATION[2..],
    );

    let fixture = plugin_fixture()?;
    let output = validator_cli(fixture.root())?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

fn plugin_fixture() -> TestResult<PluginFixture> {
    Ok(support::plugin_fixture_with_mutable_files(&[
        Path::new("agents/codexy-cartographer.toml"),
        Path::new("skills/orchestration/SKILL.md"),
        Path::new("skills/orchestration/references/classification-and-control.md"),
        Path::new("skills/orchestration/references/goal-transition-reporting.md"),
        Path::new("skills/orchestration/references/thread-and-worktree-routing.md"),
        Path::new("skills/orchestration/references/orchestration-loop.md"),
        Path::new("skills/orchestration/references/runtime-heartbeats.md"),
        Path::new("skills/orchestration/references/parent-stop-preflight.md"),
        Path::new("skills/orchestration/references/execution-budget.md"),
        Path::new("skills/orchestration/references/plain-language-user-replies.md"),
        Path::new("skills/orchestration/references/natural-korean-responses.md"),
    ])?)
}

fn reset_fixture_file(fixture: &PluginFixture, relative: &Path) -> TestResult<std::path::PathBuf> {
    fixture.reset_file(relative)?;
    Ok(fixture.root().join(relative))
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    support::validator(plugin_root, "--check-roles")
}

fn validator_cli(plugin_root: &Path) -> TestResult<Output> {
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

fn registered_orchestration_references() -> TestResult<Vec<String>> {
    let skill = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?;
    let references = skill
        .split_once("## Read Next")
        .and_then(|(_, remainder)| remainder.split_once("## Classification Gate"))
        .map(|(section, _)| section)
        .ok_or("orchestration Read Next section")?;
    Ok(references
        .lines()
        .filter_map(|line| line.split('`').nth(1))
        .filter(|path| path.starts_with("references/") && path.ends_with(".md"))
        .map(|path| format!("skills/orchestration/{path}"))
        .collect())
}
