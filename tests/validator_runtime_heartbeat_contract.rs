use std::fs;

#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;
use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const ORCHESTRATION_CLAUSES: &[&str] = &[
    "search the callable tool surface for `automation_update`",
    "register a thread-targeted `kind=heartbeat`",
    "creation MUST use `destination=\"thread\"`",
    "automation id, target thread, bounded schedule, stable observed-state identity, eligible material events, and terminal delete/disable action",
    "prompt MUST suppress unchanged observations and MUST wake the owner only for a material gate change or an explicit user/parent message",
    "MUST end its active goal and plan before waiting",
    "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
    "MUST consume the event in the same turn",
    "MUST delete or disable the heartbeat when no further observation is required",
    "MUST record the exact discovery/exposure evidence and use a bounded fallback",
    "without fabricating a monitor identity",
    "MUST mark automation id, schedule, and lifecycle as not-created",
    "MUST NOT fold a live packaged Sentinel into heartbeat observation",
    "read-only, event-driven, and subject to its no-poll/no-message boundary",
];

const TOKEN_CLAUSES: &[&str] = &[
    "polling/monitoring MUST be reserved for an observation bound to one complete runtime-issued monitor identity",
    "heartbeat route MUST bind the observation to its heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "heartbeat route MUST NOT require a persistent exec/session identifier or same-process resume",
    "separate process-backed monitor MUST bind the observation to a persistent runtime monitor or wait session id, a scheduled next-observation time or deadline, the last observed state fingerprint or event identity, and same-process resume",
    "without either complete runtime-issued identity are continuation turns, not polling",
    "bounded schedule, state fingerprint, material-event set, and delete/disable state",
    "MUST suppress unchanged observations",
    "material gate change or an explicit user/parent message",
    "active goal and plan MUST end before runtime-owned waiting",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
];

const TEMPLATE_CLAUSES: &[&str] = &[
    "callable discovery/exposure evidence:",
    "heartbeat automation id:",
    "target thread:",
    "bounded schedule:",
    "state fingerprint:",
    "eligible material events:",
    "unchanged observations suppressed:",
    "terminal delete/disable action:",
];

const TRANSITION_CLAUSES: &[&str] = &[
    "heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "MUST NOT require a persistent exec/session identifier or same-process resume",
    "persistent exec/session identifier, a scheduled next-observation deadline, the last observed state fingerprint or event identity, and same-process resume",
];

#[test]
fn plugin_fixture_reset_restores_a_mutated_skill() -> TestResult {
    let fixture = support::plugin_fixture()?;
    let relative = std::path::Path::new("skills/token-efficient-orchestration/SKILL.md");
    let path = fixture.root().join(relative);
    let original = fs::read_to_string(&path)?;

    fs::write(&path, "mutated fixture")?;
    fixture.reset_file(relative)?;

    assert_eq!(fs::read_to_string(path)?, original);
    Ok(())
}

#[test]
fn validator_requires_runtime_heartbeat_contract() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let heartbeat = fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/runtime-heartbeats.md"),
    )?;
    let token = fs::read_to_string(
        root.join("plugins/codexy/skills/token-efficient-orchestration/SKILL.md"),
    )?;

    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&heartbeat),
        structured_contract_rules::HEARTBEAT,
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&token),
        structured_contract_rules::TOKEN_CONTAINMENT,
    );
    assert_rejected_clauses(
        "skills/codex-orchestration/references/runtime-heartbeats.md",
        ORCHESTRATION_CLAUSES,
        "removed heartbeat policy",
        "runtime heartbeat contract",
    )?;
    assert_rejected_clauses(
        "skills/token-efficient-orchestration/SKILL.md",
        TOKEN_CLAUSES,
        "removed heartbeat policy",
        "runtime heartbeat contract",
    )?;
    assert_rejected_clauses(
        "skills/token-efficient-orchestration/templates/delta-poll.md",
        TEMPLATE_CLAUSES,
        "removed heartbeat slot",
        "runtime heartbeat",
    )?;
    assert_rejected_clauses(
        "skills/codex-orchestration/references/goal-transition-reporting.md",
        TRANSITION_CLAUSES,
        "removed monitor identity policy",
        "monitor identities",
    )?;
    Ok(())
}

fn assert_rejected_clauses(
    relative: &str,
    clauses: &[&str],
    replacement: &str,
    expected_error: &str,
) -> TestResult {
    let fixture = support::plugin_fixture()?;
    let relative = std::path::Path::new(relative);
    let path = fixture.root().join(relative);
    for clause in clauses {
        fixture.reset_file(relative)?;
        let original = fs::read_to_string(&path)?;
        fs::write(&path, original.replace(clause, replacement))?;
        let output = support::validator_instruction_policy(fixture.root())?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains(expected_error));
    }
    Ok(())
}

#[test]
fn validator_rejects_weak_runtime_heartbeat_policy() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/references/runtime-heartbeats.md");
    let original = fs::read_to_string(&path)?;
    for replacement in [
        "\n## MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat",
        "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat is not required.",
    ] {
        fs::write(
            &path,
            original.replace(
                "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat",
                replacement,
            ),
        )?;
        assert!(
            !support::validator_instruction_policy(&plugin_root)?
                .status
                .success()
        );
    }

    let clause = "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat";
    fs::write(
        &path,
        original.replace(
            clause,
            &format!(
                "removed heartbeat policy\n\n## Historical Example\nThis policy was retired. {clause}."
            ),
        ),
    )?;
    assert!(
        !support::validator_instruction_policy(&plugin_root)?
            .status
            .success(),
        "validator accepted a required clause from historical prose"
    );

    for heading in ["Current Policy", "Non-Historical Requirements"] {
        fs::write(
            &path,
            original.replace(
                clause,
                &format!(
                    "removed heartbeat policy\n\n## Historical Example\nThis policy was retired.\n\n## {heading}\n{clause}."
                ),
            ),
        )?;
        let output = support::validator_instruction_policy(&plugin_root)?;
        assert!(
            output.status.success(),
            "validator ignored active policy under {heading:?}: {}",
            support::stderr(&output)
        );
    }

    fs::write(
        &path,
        format!(
            "{original}\nThe owner MAY fold a live packaged Sentinel into heartbeat observation.\n"
        ),
    )?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("must not permit Sentinel"));

    fs::write(
        &path,
        format!(
            "{original}\n## Historical Example\nThis old policy is retained for context. The owner MAY fold a live packaged Sentinel into heartbeat observation.\n"
        ),
    )?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator rejected historical-only Sentinel wording: {}",
        support::stderr(&output)
    );

    fs::write(
        &path,
        format!(
            "{original}\n## Historical Example\nThis old policy is retained for context.\n## Current Policy\nThe owner MAY fold a live packaged Sentinel into heartbeat observation.\n"
        ),
    )?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("must not permit Sentinel"));
    Ok(())
}
