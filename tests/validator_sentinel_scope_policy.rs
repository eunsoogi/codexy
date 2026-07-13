mod support;

use support::{TestResult, copy_plugin_fixture, stderr, validator};

const SENTINEL_SCOPE_CLAUSES: &[&str] = &[
    "MUST review only this issue's acceptance criteria, authorized behavior/files, current PR head or current diff, and necessary regressions.",
    "Every BLOCK finding MUST map to an in-scope acceptance criterion.",
    "Unrelated edge cases MUST be documented as non-blocking follow-up issues and MUST NOT block this lane.",
    "Recurring same-class defects MUST receive one structural root-cause repair rather than phrase patches; MUST ask parent before widening files.",
];

const CONTRADICTORY_SCOPE_POLICY: &[&str] = &[
    "Sentinel MAY BLOCK on unrelated edge cases.",
    "Sentinel SHOULD BLOCK on unrelated edge cases.",
    "Sentinel MUST NOT ignore safety, but MAY BLOCK on unrelated edge cases.",
    "Sentinel MUST NOT ignore safety and MAY BLOCK on unrelated edge cases.",
    "Sentinel MAY BLOCK, including on unrelated edge cases.",
    "Sentinel MAY review authorized behavior and unrelated files.",
    "Sentinel MAY BLOCK necessary regressions and unrelated edge cases.",
    "Sentinel MAY review unrelated behaviors or files.",
    "Sentinel is allowed to review unrelated behaviors or files.",
    "Recurring same-class defects MAY use phrase patches instead of a structural root-cause repair.",
    "Recurring same-class defects MAY be resolved with phrase patches.",
];

const SCOPE_POLICY_PROHIBITIONS: &[&str] = &[
    "Sentinel MUST NOT BLOCK on unrelated edge cases.",
    "Sentinel MUST NOT review unrelated behaviors or files.",
    "Recurring same-class defects MUST NOT use phrase patches.",
    "Sentinel MAY review authorized files but MUST NOT BLOCK unrelated edge cases.",
    "Sentinel MAY review authorized files and MUST NOT BLOCK unrelated edge cases.",
    "Sentinel MAY review authorized behavior and files and MUST NOT BLOCK unrelated edge cases.",
];

const LIVE_SENTINEL_CONTROLS: &[&str] = &[
    "Root MAY poll a live Sentinel.",
    "A child owner MAY message a live Sentinel.",
    "Root MAY interrupt or replace a live Sentinel.",
    "A child owner MAY follow up with a live Sentinel.",
    "Root MUST NOT ignore safety, but MAY poll a live Sentinel.",
    "Root MAY send a terminal-status request to a live Sentinel.",
];

const LIVE_SENTINEL_EXEMPTIONS: &[&str] = &[
    "Generic child and ledger polling MAY continue.",
    "Root MUST NOT poll a live Sentinel.",
    "Historically, root MAY poll a live Sentinel.",
    "After terminal PASS, root MAY poll a live Sentinel's archived result.",
    "sentinel_policy: Root MAY poll a live Sentinel.",
    "```text\nRoot MAY poll a live Sentinel.\n```",
];

const LIVE_OBSERVATION_SKILLS: &[&str] = &[
    "codex-orchestration",
    "proof-driven-completion",
    "token-efficient-orchestration",
];

const LIVE_OBSERVATION_CLAUSE: &str =
    "Live Sentinel observation MUST be read-only and event-driven.";

#[test]
fn validator_cli_rejects_missing_sentinel_scope_policy() -> TestResult {
    for clause in SENTINEL_SCOPE_CLAUSES {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
        let agent = std::fs::read_to_string(&agent_path)?;
        std::fs::write(&agent_path, agent.replace(clause, "removed policy clause."))?;

        let output = validator(&plugin_root, "--check-roles")?;
        assert!(!output.status.success(), "{clause:?} was not enforced");
        assert!(stderr(&output).contains("scope policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_orchestration_scope_policy() -> TestResult {
    for clause in SENTINEL_SCOPE_CLAUSES {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, skill.replace(clause, "removed policy clause."))?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{clause:?} was not enforced");
        assert!(stderr(&output).contains("scope policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_contradictory_sentinel_scope_policy() -> TestResult {
    for contradiction in CONTRADICTORY_SCOPE_POLICY {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
        let agent = std::fs::read_to_string(&agent_path)?;
        let agent = agent.replacen("\n\"\"\"\n", &format!("\n{contradiction}\n\"\"\"\n"), 1);
        std::fs::write(&agent_path, agent)?;

        let output = validator(&plugin_root, "--check-roles")?;
        assert!(!output.status.success(), "{contradiction:?} was permitted");
        assert!(stderr(&output).contains("scope policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_contradictory_orchestration_scope_policy() -> TestResult {
    for contradiction in CONTRADICTORY_SCOPE_POLICY {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{skill}\n{contradiction}\n"))?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{contradiction:?} was permitted");
        assert!(stderr(&output).contains("scope policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_scope_policy_prohibitions() -> TestResult {
    for prohibition in SCOPE_POLICY_PROHIBITIONS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
        let agent = std::fs::read_to_string(&agent_path)?;
        let agent = agent.replacen("\n\"\"\"\n", &format!("\n{prohibition}\n\"\"\"\n"), 1);
        std::fs::write(&agent_path, agent)?;
        let output = validator(&plugin_root, "--check-roles")?;
        assert!(
            output.status.success(),
            "{prohibition:?}: {}",
            stderr(&output)
        );

        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{skill}\n{prohibition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            output.status.success(),
            "{prohibition:?}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_live_sentinel_controls_but_allows_exemptions() -> TestResult {
    for control in LIVE_SENTINEL_CONTROLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{skill}\n{control}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{control:?} was permitted");
    }
    for exemption in LIVE_SENTINEL_EXEMPTIONS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{skill}\n{exemption}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert!(
            output.status.success(),
            "{exemption:?}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_keeps_live_observation_out_of_sentinel_role_scope() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
    let agent = std::fs::read_to_string(&agent_path)?;
    let agent = agent.replacen(
        "\n\"\"\"\n",
        "\nRoot MAY poll a live Sentinel.\n\"\"\"\n",
        1,
    );
    std::fs::write(&agent_path, agent)?;
    let output = validator(&plugin_root, "--check-roles")?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_or_contradictory_live_policy_in_every_skill() -> TestResult {
    for skill in LIVE_OBSERVATION_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let path = plugin_root.join(format!("skills/{skill}/SKILL.md"));
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(
            &path,
            text.replace(LIVE_OBSERVATION_CLAUSE, "removed clause."),
        )?;
        assert!(
            !validator(&plugin_root, "--check")?.status.success(),
            "{skill}"
        );

        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let path = plugin_root.join(format!("skills/{skill}/SKILL.md"));
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(&path, format!("{text}\nRoot MAY poll a live Sentinel.\n"))?;
        assert!(
            !validator(&plugin_root, "--check")?.status.success(),
            "{skill}"
        );
    }
    Ok(())
}
