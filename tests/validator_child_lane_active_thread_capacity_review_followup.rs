use std::process::{Command, Output};

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}

#[test]
fn validator_rejects_creation_after_active_count_refresh_reaches_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should project from the latest active-only count before creation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_splits_compact_codex_app_clauses() -> Result<(), Box<dyn std::error::Error>> {
    for action in [
        "also created child Codex app thread",
        "also started child Codex app thread",
        "also forked child Codex app thread",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex app threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child Codex app thread thread-269 for issue #269, {action} thread-270 for issue #270.
Maintainer reassignment: none
"#
        ))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success()
                && stderr.contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity compact `{action}` operations, got:\n{stderr}"
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_compact_codex_app_operation_clauses_with_prior_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: existing owner thread thread-old found for issue #270.
Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #270.
Thread creation: created child thread thread-269 for issue #269 and also created a replacement child thread thread-new for issue #270.
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept compact same-line operations when prior count and owner lookup evidence covers them, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_compact_same_issue_creations_with_single_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269-a for issue #269 and also created child thread thread-269-b for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should require fresh lookup evidence before a second same-issue creation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing fresh owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_keeps_aggregate_total_after_stale_active_component()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Active/waiting child Codex threads: 5
Waiting child Codex threads: 1
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep the aggregate cap after stale component counts"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_same_line_freed_count_spent_twice() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Child thread thread-old stopped and freed replacement capacity; Active child Codex threads: 4; Existing issue/PR owner check: existing owner thread thread-old found for issue #269; Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269; Thread creation: created replacement child thread thread-new for issue #269; Existing issue/PR owner check: no existing owner thread found for issue #270; Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a later same-line operation reuse a freed count already spent by replacement"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_passive_created_non_prefixed_thread_id_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Child thread 019ef was created for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should detect passive creations that name non-prefixed Codex thread IDs"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity passive creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_negated_passive_non_prefixed_thread_creation()
-> Result<(), Box<dyn std::error::Error>> {
    for line in [
        "Child thread 019ef was not created for issue #269.",
        "No child thread 019ef was created for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{line}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should ignore negated passive non-prefixed creation wording `{line}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_creation_before_negated_passive_creation_clause()
-> Result<(), Box<dyn std::error::Error>> {
    for separator in [",", "and", "but"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269 {separator} child thread 019ef was not created for issue #270.
Maintainer reassignment: none
"#
        ))?;

        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity creation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_creation_before_no_passive_creation_clause()
-> Result<(), Box<dyn std::error::Error>> {
    for separator in [",", "and", "but"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269 {separator} no child thread 019ef was created for issue #270.
Maintainer reassignment: none
"#
        ))?;

        assert!(output.status.success());
    }
    Ok(())
}
