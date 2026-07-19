use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_allows_found_none_owner_lookup_for_new_child_thread()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread found: none for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should treat found:none owner-check fields as no-owner evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_continued_owner_reuse_without_repeating_thread_id()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Thread resume: continued child thread for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow issue-matched owner reuse without repeating the thread id, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_counts_rate_limited_child_lanes_as_waiting() -> Result<(), Box<dyn std::error::Error>>
{
    for count in [
        "Active/waiting child Codex threads: 4 active, 2 rate-limited",
        "Active/waiting child Codex threads: 4 active, 2 rate limited",
        "Active/waiting child Codex threads: 4 active, rate-limited 2",
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
            "validator should count rate-limited child lanes as waiting for `{count}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads")
                || String::from_utf8_lossy(&output.stderr)
                    .contains("keep at most five active child Codex threads"),
            "stderr should name over-capacity rate-limited count `{count}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_resets_stale_waiting_with_blocked_alias_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    for refreshed_count in [
        "Active child Codex threads: 4, 0 blocked",
        "Active child Codex threads: 4, zero passive",
        "Active child Codex threads: 4, 0 rate-limited",
        "Active child Codex threads: 4, zero rate limited",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Waiting child Codex threads: 1
{refreshed_count}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should reset stale waiting state for `{refreshed_count}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_new_owner_after_found_lookup_clears_stale_no_owner_id()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Existing issue/PR owner check: no existing owner thread found for PR #300.
Thread creation: created child thread thread-new for issue #269 / PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not combine stale no-owner issue ids with later no-owner PR ids after a found owner lookup"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should require reuse or old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_old_owner_disposition_matching_either_issue_or_pr()
-> Result<(), Box<dyn std::error::Error>> {
    for disposition in [
        "Old owner for issue #269 was stopped.",
        "Old owner for PR #300 was stopped.",
        "Existing owner thread thread-old was superseded.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269 / PR #300.
{disposition}
Thread creation: created replacement child thread thread-new for issue #269 / PR #300.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should accept old-owner disposition matched by either issue, PR, or thread id for `{disposition}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
