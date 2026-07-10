use std::path::Path;
use std::process::{Command, Output};

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
        assert!(role.contains(NO_RECURSIVE_DELEGATION), "{role}");
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
fn validator_rejects_recursive_or_missing_reference_delegation_contract() -> TestResult {
    for (relative_path, replacement, expected) in [
        (
            "skills/codex-orchestration/SKILL.md",
            "MAY create an additional reviewer task.",
            "permits recursive delegation",
        ),
        (
            "skills/codex-orchestration/references/classification-and-control.md",
            "No delegation boundary is recorded.",
            "missing required boundary text",
        ),
        (
            "skills/codex-orchestration/references/orchestration-loop.md",
            "No delegation boundary is recorded.",
            "missing required boundary text",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        support::copy_dir(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let path = plugin_root.join(relative_path);
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(&path, text.replace(NO_RECURSIVE_DELEGATION, replacement))?;

        let output = validator(&plugin_root)?;

        assert!(!output.status.success(), "{relative_path}");
        assert!(stderr(&output).contains(expected), "{relative_path}");
    }
    Ok(())
}

#[test]
fn validator_rejects_recursive_permission_appended_to_canonical_child_clause() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\nA child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers, and those helpers MAY spawn another helper.\n",
    );
    std::fs::write(&skill_path, skill)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("permits recursive delegation"));
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

    assert!(orchestration.contains(
        "A child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers."
    ));
    assert!(orchestration.contains(NO_RECURSIVE_DELEGATION));
    assert!(loop_reference.contains(
        "Every helper or Sentinel assignment MUST include the nonrecursive delegation prohibition."
    ));
    assert!(loop_reference.contains(NO_RECURSIVE_DELEGATION));
    for example in [
        "spawn_agent(agent_type=\"codexy-sentinel\", message=\"Review the current diff, exact head, scope, verification output, and evidence. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
        "spawn_agent(agent_type=\"codexy-pathfinder\", message=\"Produce an atomic plan and verification checklist. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
        "spawn_agent(agent_type=\"codexy-cartographer\", message=\"Map the relevant files. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
    ] {
        assert!(orchestration.contains(example), "{example}");
    }
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
