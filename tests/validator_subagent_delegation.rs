use std::path::Path;
use std::process::{Command, Output};

#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;
mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NO_RECURSIVE_DELEGATION: &str = "MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.";

#[test]
fn validator_accepts_all_packaged_roles_with_nonrecursive_delegation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
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
            &structured_contract::Contract::markdown(&role),
            &structured_contract_rules::DELEGATION[..1],
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_role_without_nonrecursive_delegation_prohibition() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let role_path = plugin_root.join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replace(NO_RECURSIVE_DELEGATION, "MUST NOT recurse through helpers."),
    )?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("nonrecursive delegation contract is missing"));
    Ok(())
}

#[test]
fn validator_rejects_role_that_permits_recursive_delegation() -> TestResult {
    for permission in [
        "MAY spawn another helper after mapping the repository.",
        "CAN spawn a helper after mapping the repository.",
        "MAY delegate work to a helper after mapping the repository.",
        "Allowed actions: spawn helper tasks after mapping the repository.",
        "A helper is allowed to spawn another helper after mapping the repository.",
        "Allowed actions: spawn helper tasks, but MUST NOT merge.",
        "Permitted to delegate to a reviewer thread after mapping the repository.",
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        support::copy_dir(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let role_path = plugin_root.join("agents/codexy-cartographer.toml");
        let role = std::fs::read_to_string(&role_path)?;
        std::fs::write(
            &role_path,
            role.replacen("\n\"\"\"", &format!("\n{permission}\n\"\"\""), 1),
        )?;

        let output = validator(&plugin_root)?;

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
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replace(
            "A child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers.",
            "A child implementation thread MAY not spawn first-level helpers.",
        ),
    )?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("nonrecursive delegation contract is missing"));
    Ok(())
}

#[test]
fn validator_rejects_recursive_permission_in_orchestration_skill() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        &path,
        text.replace(
            NO_RECURSIVE_DELEGATION,
            "MAY create an additional reviewer task.",
        ),
    )?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("permits recursive delegation"));
    Ok(())
}

#[test]
fn validator_rejects_recursive_permission_in_every_registered_reference() -> TestResult {
    for relative_path in registered_orchestration_references()? {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        support::copy_dir(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let path = plugin_root.join(&relative_path);
        let mut text = std::fs::read_to_string(&path)?;
        text.push_str("\nA helper MAY spawn another helper.\n");
        std::fs::write(path, text)?;

        let output = validator(&plugin_root)?;

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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let loop_reference = std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/orchestration-loop.md"),
    )?;

    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&orchestration),
        &structured_contract_rules::DELEGATION[1..2],
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&loop_reference),
        &structured_contract_rules::DELEGATION[2..],
    );
    Ok(())
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

fn registered_orchestration_references() -> TestResult<Vec<String>> {
    let skill = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
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
        .map(|path| format!("skills/codex-orchestration/{path}"))
        .collect())
}
