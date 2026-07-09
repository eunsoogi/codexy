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
fn validator_counts_active_thread_ids_with_labeled_zero_waiting()
-> Result<(), Box<dyn std::error::Error>> {
    for waiting in ["zero waiting", "0 waiting"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101, thread-102, thread-103, thread-104, thread-105, {waiting}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should count active thread IDs before labeled {waiting:?}"
        );
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
fn validator_adds_labeled_waiting_blocked_and_passive_counts_to_active_ids()
-> Result<(), Box<dyn std::error::Error>> {
    for component in ["1 waiting", "1 blocked", "1 passive"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101, thread-102, thread-103, thread-104, thread-105, {component}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should add labeled {component:?} count to listed active thread IDs"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("keep at most five active child Codex threads"),
            "stderr should name over-capacity ledger, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_explicit_total_with_active_ids_and_labeled_waiting()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101, thread-102, thread-103, thread-104, thread-105, 4 total, zero waiting
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve explicit total evidence over listed tokens, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_multiline_list_with_only_labeled_waiting_as_missing_count()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads:
- thread-101
- thread-102
- thread-103
- thread-104
- thread-105
- zero waiting
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let unsupported multiline ledgers project from a zero waiting label"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("require evidence of the active child"),
        "stderr should require same-line active-count evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_counts_numbered_same_line_thread_ids_with_labeled_waiting()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 1. thread-101, 2. thread-102, 3. thread-103, 4. thread-104, 5. thread-105, zero waiting
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should count numbered same-line thread IDs before a labeled waiting count"
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
fn validator_counts_numbered_same_line_thread_ids_before_ordinals()
-> Result<(), Box<dyn std::error::Error>> {
    for ledger in [
        "Active child Codex threads: 1. thread-101, 2. thread-102, 3. thread-103, 4. thread-104, 5. thread-105",
        "Active child Codex threads: 1) thread-a1, 2) thread-b2, 3) thread-c3, 4) thread-d4, 5) thread-e5, 6) thread-f6",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
{ledger}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should count numbered thread IDs before the first ordinal for `{ledger}`"
        );
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
fn validator_preserves_explicit_leading_total_before_numbered_examples()
-> Result<(), Box<dyn std::error::Error>> {
    for ledger in [
        "Active child Codex threads: 5, including 1. thread-old for issue #269, 2. thread-extra for issue #270.",
        "Active child Codex threads: 5 thread IDs include 1. thread-old for issue #269, 2. thread-extra for issue #270.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
{ledger}
Existing issue/PR owner check: no existing owner thread found for issue #271.
Thread creation: created child thread thread-new for issue #271.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should not let numbered examples override an explicit leading total for `{ledger}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity creation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
