#[test]
fn codex_orchestration_requires_situational_specialist_routing()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let control = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/codex-orchestration/references/classification-and-control.md",
    ))?;
    let loop_ref = std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/orchestration-loop.md"),
    )?;
    let classification =
        std::fs::read_to_string(root.join("plugins/codexy/skills/task-classification/SKILL.md"))?;
    let skill_flat = skill.split_whitespace().collect::<Vec<_>>().join(" ");
    let control_flat = control.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(skill.contains("the owning thread MUST use that\nspecialist"));
    assert!(skill.contains("task clearly falls\nwithin that specialist's stated scope"));
    assert!(skill.contains("concrete skip rationale tied"));
    assert!(skill.contains("A generic \"not needed\" note is\ninsufficient."));
    assert!(skill.contains("subagent MUST NOT be treated as a\nCodex subthread/worktree owner"));
    assert!(loop_ref.contains("whose stated scope clearly matches the task"));
    assert!(control.contains("It MUST NOT replace a"));
    assert!(control.contains("required Codex child thread/worktree owner"));
    assert!(
        classification
            .contains("tools/evidence MUST name the specialist roles whose stated scope clearly")
    );
    assert!(classification.contains("It MUST NOT"));
    assert!(
        classification.contains("treat specialist subagent use as the child thread/worktree owner")
    );
    assert!(skill_flat.contains(
        "MUST use `codexy-warden` for workflows, shell commands, credentials, remote MCP endpoints, untrusted input, repository permissions"
    ));
    assert!(skill_flat.contains(
        "MUST use `codexy-auditor` after implementation for acceptance-criteria, readiness, and observable verification passes"
    ));
    assert!(control_flat.contains(
        "`codexy-warden` for workflows, shell commands, credentials, remote MCP endpoints, untrusted input, repository permissions"
    ));
    assert!(control_flat.contains(
        "`codexy-auditor` after implementation for acceptance-criteria, readiness, and observable verification passes"
    ));
    for forbidden in [
        "A generic \"not needed\" note is acceptable.",
        "specialist subagent use may satisfy child thread/worktree ownership",
        "a subagent helper may replace a required Codex child thread/worktree owner",
        "`codexy-auditor` or `codexy-warden` for compliance",
        "`codexy-auditor` or `codexy-warden` for credentials",
        "`codexy-auditor` or `codexy-warden` for command",
    ] {
        assert!(!skill_flat.contains(forbidden));
        assert!(!control_flat.contains(forbidden));
        assert!(!loop_ref.contains(forbidden));
        assert!(!classification.contains(forbidden));
    }

    for specialist in [
        "codexy-cartographer",
        "codexy-pathfinder",
        "codexy-architect",
        "codexy-tracer",
        "codexy-auditor",
        "codexy-warden",
        "codexy-scribe",
        "codexy-forge",
        "codexy-weaver",
        "codexy-sculptor",
        "codexy-shipwright",
        "codexy-sentinel",
    ] {
        assert!(skill.contains(specialist));
        assert!(control.contains(specialist));
    }

    Ok(())
}
