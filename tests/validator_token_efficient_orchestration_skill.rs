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
    let transition = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/codex-orchestration/references/goal-transition-reporting.md",
    ))?;
    let normalized_skill = skill.split_whitespace().collect::<Vec<_>>().join(" ");

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
        "Suppress unchanged continuation turns",
        "MUST keep the monitor scheduled",
        "MUST NOT emit a status message or start another model turn",
        "next scheduled read-only observation MAY run at its bounded interval",
        "MUST NOT terminate or cancel the underlying wait/monitor session",
        "persistent runtime monitor or wait session id",
        "scheduled next-observation time or deadline",
        "last observed state fingerprint or event identity",
        "Distinct model/assistant turn ids",
        "continuation turns, not polling",
        "MUST NOT reschedule themselves or emit another unchanged turn",
        "send exactly one terminal handoff delta",
        "confirm task-surface delivery before the stop/archive or goal transition",
    ] {
        assert!(normalized_skill.contains(required), "missing {required:?}");
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
    let normalized_transition = transition.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_contract = format!("{normalized_orchestration} {normalized_transition}");
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
        "authorized child-local monitor that observes no qualifying event MUST keep its bounded schedule without emitting status or starting another model turn",
        "This MUST NOT terminate the underlying monitor",
        "Before stop, archive, ownership release, or `update_goal(blocked)`",
        "send exactly one terminal handoff delta to the source parent",
        "MUST NOT perform the stop/archive/blocked transition",
        "Polling/monitoring is a runtime claim, not an agent label",
        "persistent monitor or wait session identifier",
        "Repeated model/assistant turn ids",
        "classified as a continuation turn",
        "MUST NOT reschedule themselves or emit another unchanged turn",
    ] {
        assert!(
            normalized_contract.contains(required),
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
