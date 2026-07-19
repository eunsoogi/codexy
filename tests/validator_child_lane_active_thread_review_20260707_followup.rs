use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_replacement_when_count_names_different_codex_thread_id()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Child thread 019ef stopped and freed replacement capacity.
Active child Codex threads: 5, including 019ff for issue #269.
Existing issue/PR owner check: existing owner thread 019ef found for issue #269.
Old owner disposition: existing owner thread 019ef was stopped as unusable for issue #269.
Thread creation: created replacement child thread 019aa for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat a different non-prefixed thread id for the same issue as replacement capacity"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity replacement, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_mixed_old_owner_disposition_with_accepted_claim()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was not stopped but was unusable for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept mixed disposition wording when an unnegated accepted disposition is present\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
