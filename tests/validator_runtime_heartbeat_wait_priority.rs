use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REFERENCE: &str = "skills/codex-orchestration/references/runtime-heartbeats.md";
const TOKEN_SKILL: &str = "skills/token-efficient-orchestration/SKILL.md";
const HOST_FALLBACK: &str = "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata.";
const LEGACY_ELIGIBILITY: &str = "When GitHub CI, review-thread state, child state, or another external gate will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable.";
const LEGACY_REGISTRATION: &str = "The owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path.";
const DESKTOP_CONTINUITY: &str = "While a desktop-origin root turn has a callable `wait_threads` handler, the owner MUST keep ordinary child waits in a cursor-based `wait_threads` loop without finalizing that root turn between unchanged waits; mobile input that interrupts the active local wait MUST be consumed in that same local turn before the cursor-based wait continues.";
const SLINGSHOT_RECOVERY: &str = "If a slingshot-host turn still returns `No handler registered` after the one fresh discovery and one host-aware retry, the owner MUST emit exactly one unavailable evidence receipt and require desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery.";
const SLINGSHOT_HEARTBEAT_EXEMPTION: &str = "The slingshot recovery route is not an unavailable-wait fallback eligible for heartbeat registration; it ends in desktop-origin root re-entry.";
const LEGACY_THREAD_TARGETED_REGISTRATION: &str = "The owner MUST register a thread-targeted `kind=heartbeat` instead of repeated model continuations or ending without a wakeup path.";

#[test]
fn validator_accepts_ordered_wait_and_host_recovery_routes() -> TestResult {
    let fixture = support::plugin_fixture()?;
    let output = support::validator_instruction_policy(fixture.root())?;
    assert!(
        output.status.success(),
        "validator rejected ordered wait policy: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_additive_unconditional_child_state_heartbeat_conflict() -> TestResult {
    assert_rejected_addition(LEGACY_ELIGIBILITY)?;
    assert_rejected_addition(LEGACY_REGISTRATION)?;
    assert_rejected_addition(LEGACY_THREAD_TARGETED_REGISTRATION)
}

#[test]
fn validator_ignores_inactive_unconditional_child_state_history() -> TestResult {
    for addition in [
        format!("## Historical Example\n{LEGACY_ELIGIBILITY}\n{LEGACY_REGISTRATION}"),
        format!("```markdown\n{LEGACY_ELIGIBILITY}\n{LEGACY_REGISTRATION}\n```"),
        format!("## Historical Example\n{LEGACY_THREAD_TARGETED_REGISTRATION}"),
        format!("```markdown\n{LEGACY_THREAD_TARGETED_REGISTRATION}\n```"),
    ] {
        let fixture = support::plugin_fixture()?;
        let path = fixture.root().join(REFERENCE);
        fs::write(&path, format!("{}\n{addition}", fs::read_to_string(&path)?))?;
        let output = support::validator_instruction_policy(fixture.root())?;
        assert!(
            output.status.success(),
            "validator rejected inactive legacy history: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_weakened_host_transition_fallbacks() -> TestResult {
    for replacement in [
        "After a host transition failure, the owner MUST perform thread-tool discovery before fallback.",
        "After a host transition or `No handler registered` failure, the owner MUST perform one fresh thread-tool discovery before any fallback.",
        "After a host transition or `No handler registered` failure, the owner MUST perform one host-aware `wait_threads` retry before any fallback.",
        "After a host transition or `No handler registered` failure, the owner MAY use unbounded `read_thread` as a fallback.",
        "After a host transition or `No handler registered` failure, bounded metadata fallback does not consume the current parent-stage budget.",
    ] {
        assert_rejected_mutation(HOST_FALLBACK, replacement)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_weakened_desktop_and_slingshot_recovery_routes() -> TestResult {
    for relative in [REFERENCE, TOKEN_SKILL] {
        assert_rejected_policy_mutation(
            relative,
            DESKTOP_CONTINUITY,
            "The owner MAY finalize a desktop-origin root turn between ordinary child waits.",
        )?;
        assert_rejected_policy_mutation(
            relative,
            SLINGSHOT_RECOVERY,
            "The owner MAY repeat a slingshot-host wait call or schedule a heartbeat relay.",
        )?;
    }
    Ok(())
}

#[test]
fn validator_requires_slingshot_heartbeat_registration_exemption() -> TestResult {
    assert_rejected_policy_mutation(
        REFERENCE,
        SLINGSHOT_HEARTBEAT_EXEMPTION,
        "The slingshot recovery route MAY register a heartbeat relay before desktop-origin root re-entry.",
    )
}

fn assert_rejected_mutation(original_clause: &str, replacement: &str) -> TestResult {
    assert_rejected_policy_mutation(REFERENCE, original_clause, replacement)
}

fn assert_rejected_policy_mutation(
    relative: &str,
    original_clause: &str,
    replacement: &str,
) -> TestResult {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join(relative);
    let original = fs::read_to_string(&path)?;
    let mutated = original.replace(original_clause, replacement);
    assert_ne!(
        original, mutated,
        "fixture is missing ordered clause {original_clause:?}"
    );
    fs::write(path, mutated)?;
    let output = support::validator_instruction_policy(fixture.root())?;
    assert!(
        !output.status.success(),
        "validator accepted weakened wait/heartbeat priority"
    );
    assert!(support::stderr(&output).contains("runtime heartbeat contract"));
    Ok(())
}

fn assert_rejected_addition(addition: &str) -> TestResult {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join(REFERENCE);
    fs::write(&path, format!("{}\n{addition}", fs::read_to_string(&path)?))?;
    let output = support::validator_instruction_policy(fixture.root())?;
    assert!(
        !output.status.success(),
        "validator accepted additive unconditional heartbeat policy"
    );
    assert!(support::stderr(&output).contains("unconditional heartbeat"));
    Ok(())
}
