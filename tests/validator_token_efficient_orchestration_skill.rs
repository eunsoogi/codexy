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
    let receipt = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/token-efficient-orchestration/templates/session-audit-proof-receipt.json",
    ))?;
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
        "Event-driven delta",
        "event id:",
        "MUST NOT autonomously poll",
        "stable event identity",
        "stale/demoted",
        "remember",
        "refresh",
        "forget",
        "one action",
        "templates/delta-poll.md",
        "not a shortcut around `$proof-driven-completion`",
        "content equivalence",
        "codex plugin add codexy@codexy",
        "review requests",
        "review feedback",
        "child age",
        "retries per PR",
    ] {
        assert!(skill.contains(required), "missing {required:?}");
    }
    for required_slot in [
        "head SHA:",
        "base SHA:",
        "event id:",
        "event kind:",
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
    assert!(prompt.contains("event-driven"));
    assert!(!prompt.to_ascii_lowercase().contains("poll"));
    assert!(!skill.contains("installed Codexy plugin is version `1.1.0`"));
    for field in [
        "\"metadataOnly\": true",
        "\"reviewRequests\"",
        "\"reviewFeedback\"",
        "\"childAgeSeconds\"",
        "\"retriesByKind\"",
        "\"goalPlanReceipts\"",
        "\"helpers\"",
        "\"inputSha256\"",
        "\"commandReceipts\"",
    ] {
        assert!(receipt.contains(field), "receipt missing {field:?}");
    }
    assert!(orchestration.contains("$token-efficient-orchestration"));
    assert!(orchestration.contains("preserving all proof gates"));
    let normalized_orchestration = orchestration
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for required in [
        "root/orchestrator MAY end its goal and plan after dispatch",
        "child external-gate wait MUST retain active goal and plan",
        "bounded child-local monitoring",
        "send a parent delta before transition",
        "inspect archive candidates and the active reservation ledger",
        "MUST NOT archive PR owners or dirty/reserved candidates",
        "record the decision in setup evidence",
        "record the `block` and update the plan to a repair step",
        "add faithful RED coverage, repair, rerun terminal proof, then invoke exactly one fresh Sentinel review for the new file state or head",
    ] {
        assert!(
            normalized_orchestration.contains(required),
            "missing {required:?}"
        );
    }
    for required_slot in [
        "external gate wait:",
        "parent delta before transition:",
        "archive candidates inspected:",
        "active reservation ledger:",
        "archive decision:",
        "BLOCK receipt:",
        "repair plan:",
        "faithful RED coverage:",
        "terminal proof:",
        "fresh Sentinel review for new file state or head:",
    ] {
        assert!(
            template.contains(required_slot),
            "template missing {required_slot:?}"
        );
    }
    Ok(())
}
