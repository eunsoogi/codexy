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
