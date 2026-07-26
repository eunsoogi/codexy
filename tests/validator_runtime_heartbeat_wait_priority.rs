use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REFERENCE: &str = "skills/codex-orchestration/references/runtime-heartbeats.md";
const WAIT_DEFAULT: &str = "The owner MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits.";
const HEARTBEAT_RESERVATION: &str = "The owner MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable.";
const ELIGIBILITY: &str = "When genuinely scheduled monitoring or an unavailable `wait_threads` route will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable.";
const REGISTRATION: &str = "For such genuinely scheduled monitoring or unavailable-wait fallback, the owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path.";
const HOST_FALLBACK: &str = "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata.";

#[test]
fn validator_accepts_ordered_wait_and_heartbeat_routes() -> TestResult {
    let fixture = support::plugin_fixture()?;
    let policy = fs::read_to_string(fixture.root().join(REFERENCE))?;
    for clause in [
        WAIT_DEFAULT,
        HEARTBEAT_RESERVATION,
        ELIGIBILITY,
        REGISTRATION,
        HOST_FALLBACK,
    ] {
        assert!(policy.contains(clause), "missing route clause {clause:?}");
    }
    let output = support::validator_instruction_policy(fixture.root())?;
    assert!(
        output.status.success(),
        "validator rejected ordered wait policy: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_unconditional_child_state_heartbeat_conflict() -> TestResult {
    assert_rejected_mutation(
        ELIGIBILITY,
        "When GitHub CI, review-thread state, child state, or another external gate will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable.",
    )?;
    assert_rejected_mutation(
        REGISTRATION,
        "The owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path.",
    )
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

fn assert_rejected_mutation(original_clause: &str, replacement: &str) -> TestResult {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join(REFERENCE);
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
