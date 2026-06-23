#[test]
fn token_efficient_orchestration_skill_preserves_proof_gates()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/token-efficient-orchestration/SKILL.md"),
    )?;
    let prompt = std::fs::read_to_string(
        root.join("plugins/codexy/skills/token-efficient-orchestration/agents/openai.yaml"),
    )?;
    let template = std::fs::read_to_string(
        root.join("plugins/codexy/skills/token-efficient-orchestration/templates/delta-poll.md"),
    )?;
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;

    for required in [
        "Token-Efficient Orchestration",
        "Required Proof Gates",
        "current head SHA and base SHA",
        "Codex review state for the current head",
        "unresolved review thread ids",
        "not-created or not-applicable",
        "refresh existing gates",
        "Poll by delta",
        "delta since last poll",
        "stale/demoted",
        "remember",
        "refresh",
        "forget",
        "one action",
        "templates/delta-poll.md",
        "not a shortcut around `$proof-driven-completion`",
    ] {
        assert!(skill.contains(required), "missing {required:?}");
    }
    for required_slot in [
        "head SHA:",
        "base SHA:",
        "changed ids:",
        "stale or demoted:",
        "Codex review current-head state:",
        "unresolved review thread ids and outdated status:",
        "child owner evidence:",
        "merge readiness or stop condition:",
        "one next action:",
    ] {
        assert!(
            template.contains(required_slot),
            "template missing {required_slot:?}"
        );
    }
    assert!(prompt.contains("Token-Efficient Orchestration"));
    assert!(prompt.contains("$token-efficient-orchestration"));
    assert!(orchestration.contains("$token-efficient-orchestration"));
    assert!(orchestration.contains("preserving all proof gates"));
    Ok(())
}
