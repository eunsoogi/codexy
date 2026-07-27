#[test]
fn validator_rejects_classification_before_child_lane_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let evidence = "Ownership metadata source: parent-supplied\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: current-thread-owned child implementation lane\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: implement after classification\nStop/blocker: None\nLane ownership: child-owned\nChild branch codexy/231 was created after classification.\n";
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    assert!(!crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}
