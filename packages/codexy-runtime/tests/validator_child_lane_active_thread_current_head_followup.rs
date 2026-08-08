use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_codex_app_child_thread_word_order_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Created Codex app child thread thread-269 for issue #269.",
        "Codex app child thread request thread-269 for issue #269.",
        "Child thread request in Codex app: thread-269 for issue #269.",
        "Child Codex app thread request thread-269 for issue #269.",
        "Child Codex thread request thread-269 for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex app threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should treat `{operation}` as a child-thread operation"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity operation `{operation}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_ignores_negated_codex_app_child_thread_request()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex app threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
No child thread request in Codex app was made for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat negated Codex app child-thread request wording as an operation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_creation_after_labeled_active_waiting_total_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for count in [
        "Active/waiting child Codex threads: 4 active, 2 waiting",
        "Active/waiting child Codex threads: active 4, waiting 2",
        "Active/waiting child Codex threads: 4 currently active, 2 waiting",
        "Active/waiting child Codex threads: active child threads: 4, waiting child threads: 2",
        "Active/waiting child Codex threads: 4 active, 1 pending",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
{count}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Child thread created: thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should sum labeled active/waiting/pending counts before creation for `{count}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads")
                || String::from_utf8_lossy(&output.stderr)
                    .contains("keep at most five active child Codex threads"),
            "stderr should name over-capacity labeled count `{count}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_does_not_sum_unrelated_numeric_context_in_labeled_count()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 4 active, issue #269, PR #300
Existing issue/PR owner check: no existing owner thread found for issue #270.
Child thread created: thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should ignore unrelated issue/PR numbers in labeled active/waiting values\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_resets_stale_waiting_when_active_line_refreshes_waiting_component()
-> Result<(), Box<dyn std::error::Error>> {
    for refreshed_count in [
        "Active child Codex threads: 5, zero waiting",
        "Active child Codex threads: 4, 1 waiting",
        "Active child Codex threads: 4, 1 pending",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Waiting child Codex threads: 1
{refreshed_count}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should replace stale waiting counts for `{refreshed_count}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_projects_from_refreshed_active_waiting_value_without_stale_waiting()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Waiting child Codex threads: 1
Active child Codex threads: 4, zero waiting
Existing issue/PR owner check: no existing owner thread found for issue #269.
Child thread created: thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should project creation from refreshed active/waiting value without stale waiting\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_passive_started_forked_requested_child_threads_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Child thread started: thread-269 for issue #269.",
        "Child thread forked: thread-269 for issue #269.",
        "Child thread requested: thread-269 for issue #269.",
        "Child thread thread-269 was started for issue #269.",
        "Child thread thread-269 was forked for issue #269.",
        "Child thread thread-269 was requested for issue #269.",
        "Child thread thread-269 started for issue #269.",
        "Child thread thread-269 forked for issue #269.",
        "Child thread thread-269 requested for issue #269.",
        "Child thread started for issue #269.",
        "Child thread forked for issue #269.",
        "Child thread requested for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should treat passive operation `{operation}` as a child-thread launch"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity passive operation `{operation}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_ignores_negated_passive_started_forked_requested_child_threads()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Child thread thread-269 was not started for issue #269.",
        "Child thread thread-269 was not forked for issue #269.",
        "Child thread thread-269 was not requested for issue #269.",
        "Child thread thread-269 not started for issue #269.",
        "Child thread thread-269 not forked for issue #269.",
        "Child thread thread-269 not requested for issue #269.",
        "Child thread thread-269 was not yet started for issue #269.",
        "Child thread thread-269 has not been forked for issue #269.",
        "Child thread thread-269 has not been requested for issue #269.",
        "Child thread was not yet started for issue #269.",
        "Child thread has not been requested for issue #269.",
        "No child thread started: thread-269 for issue #269.",
        "No child thread forked: thread-269 for issue #269.",
        "No child thread requested: thread-269 for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should ignore negated passive operation `{operation}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
