//! Capsules created before the skill move must survive the policy change.

use crate::support::TestResult;
use codexy_runtime::validation::{
    HandoffEnvelope, StableClassification, StableHandoff, StructuredClassification,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{CapsuleFixture, bridge, stable, volatile};

const PREVIOUS: &str = "sha256:54c9f40fdc617c4e6dc3082cf5f19ef7e7c6babcf76e7551b56069a2c0bd8605";

fn install_previous(
    fixture: &CapsuleFixture,
    mut legacy: StableHandoff,
    policy: &str,
) -> TestResult {
    let mut capsule: Value = serde_json::from_slice(&std::fs::read(&fixture.capsule)?)?;
    let mut compact: Value = serde_json::from_str(capsule["envelope"].as_str().unwrap_or(""))?;
    legacy.policy_digest = policy.into();
    compact["stable_identity"] = format!(
        "codexy.handoff.stable.v1:{:x}",
        Sha256::digest(serde_json::to_vec(&legacy)?)
    )
    .into();
    capsule["envelope"] = serde_json::to_string(&compact)?.into();
    std::fs::write(&fixture.capsule, serde_json::to_vec(&capsule)?)?;
    let mut authority: Value = serde_json::from_slice(&std::fs::read(&fixture.authority)?)?;
    authority["stable"] = serde_json::to_value(legacy)?;
    std::fs::write(&fixture.authority, serde_json::to_vec(&authority)?)?;
    Ok(())
}

#[test]
fn previous_policy_capsule_retains_live_authority_checks() -> TestResult {
    let fixture = CapsuleFixture::new("compaction")?;
    // Exact context-tier policy shipped immediately before the skill move.
    install_previous(&fixture, stable(), PREVIOUS)?;
    let output = fixture.command(bridge()).output()?;
    assert!(
        output.status.success(),
        "previous capsule rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !fixture.command(bridge()).status()?.success(),
        "replay must fail"
    );
    Ok(())
}

#[test]
fn previous_policy_never_relaxes_live_authority_or_accepts_unknown_policy() -> TestResult {
    let forged = CapsuleFixture::new("compaction")?;
    install_previous(&forged, stable(), PREVIOUS)?;
    let mut authority: Value = serde_json::from_slice(&std::fs::read(&forged.authority)?)?;
    authority["currentHead"] = "different-head".into();
    std::fs::write(&forged.authority, serde_json::to_vec(&authority)?)?;
    assert!(!forged.command(bridge()).status()?.success());

    let unknown = CapsuleFixture::new("compaction")?;
    install_previous(&unknown, stable(), "sha256:unknown")?;
    assert!(!unknown.command(bridge()).status()?.success());
    Ok(())
}

#[test]
fn old_wiki_route_and_new_route_share_replay_without_reactivating_old_skill() -> TestResult {
    let fixture = CapsuleFixture::new("compaction")?;
    let mut old = stable();
    old.task_classification = StableClassification::Structured(StructuredClassification {
        workflow: "implementation".into(),
        surfaces: vec!["research/wiki".into()],
        risks: vec![],
    });
    old.selected_references.push("dreaming".into());
    install_previous(&fixture, old.clone(), PREVIOUS)?;
    assert!(fixture.command(bridge()).status()?.success());

    let mut new = old;
    *new.selected_references.last_mut().expect("wiki route") = "self-improvement".into();
    let event = volatile(
        "fresh-child-continuation",
        "fresh-child",
        "child-679",
        "parent-679",
        "child-679",
        false,
    );
    let envelope = HandoffEnvelope::new(new, event).canonical_json()?;
    let mut capsule: Value = serde_json::from_slice(&std::fs::read(&fixture.capsule)?)?;
    capsule["consumer"] = "fresh-child".into();
    capsule["envelope"] = envelope.into();
    std::fs::write(&fixture.capsule, serde_json::to_vec(&capsule)?)?;
    assert!(fixture.command(bridge()).status()?.success());
    Ok(())
}

#[test]
fn rollback_export_keeps_original_replay_and_rejects_conflicting_authority() -> TestResult {
    let fixture = CapsuleFixture::new("compaction")?;
    assert!(fixture.command(bridge()).status()?.success());
    let directory = fixture._temporary.path().canonicalize()?;
    let replay = directory.join("replay.json");
    let authority_path = fixture.authority.canonicalize()?;
    let exported = directory.join("previous-replay.json");
    let before = std::fs::read(&replay)?;
    let run = |output: &std::path::Path| {
        std::process::Command::new(bridge())
            .args(["--export-previous-replay", "--replay"])
            .arg(&replay)
            .arg("--authority")
            .arg(&authority_path)
            .arg("--output")
            .arg(output)
            .output()
    };
    let result = run(&exported)?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(&replay)?, before);
    let projected: Vec<String> = serde_json::from_slice(&std::fs::read(&exported)?)?;
    let original: Vec<String> = serde_json::from_slice(&before)?;
    let projected: Value = serde_json::from_str(&projected[0])?;
    let original: Value = serde_json::from_str(&original[0])?;
    assert_eq!(projected["volatile"], original["volatile"]);
    assert_ne!(projected["stable_identity"], original["stable_identity"]);
    assert!(
        !run(&exported)?.status.success(),
        "must not overwrite an export"
    );

    let mut authority: Value = serde_json::from_slice(&std::fs::read(&fixture.authority)?)?;
    authority["currentHead"] = "wrong-head".into();
    std::fs::write(&fixture.authority, serde_json::to_vec(&authority)?)?;
    let rejected = directory.join("rejected.json");
    assert!(!run(&rejected)?.status.success());
    assert!(!rejected.exists());
    Ok(())
}
