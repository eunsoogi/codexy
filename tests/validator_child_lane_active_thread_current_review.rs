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
fn validator_allows_disposition_pr_match_when_issue_hash_is_also_present()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread found for PR #300.
Old owner disposition: existing owner thread was stopped as unusable for issue #269 / PR #300.
Thread creation: created replacement child thread thread-300 for issue #269 / PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should match PR-scoped old-owner disposition when the disposition also mentions the linked issue\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_passive_no_owner_found_result() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread was found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should classify passive no-owner result wording as NotFound\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_evidence_before_passive_owner_words()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing evidence that owner thread was found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat missing-evidence wording as a no-owner lookup result"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_evidence_with_passive_no_owner_words()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: missing evidence that no existing owner thread was found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat missing-evidence passive wording as a no-owner lookup result"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_without_evidence_with_passive_no_owner_words()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: without evidence that no existing owner thread was found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat without-evidence passive wording as a no-owner lookup result"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_no_evidence_with_passive_no_owner_words()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no evidence that no existing owner thread was found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat no-evidence passive wording as a no-owner lookup result"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_never_stopped_old_owner_disposition() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old never stopped for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat never-stopped wording as old-owner disposition"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing old-owner disposition evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_wasnt_stopped_old_owner_disposition() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old wasn't stopped for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat wasn't-stopped wording as old-owner disposition"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing old-owner disposition evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_new_owner_when_mixed_lookup_line_finds_pr_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269; existing owner thread thread-300 found for PR #300.
Thread creation: created child thread thread-new for PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should preserve the found PR owner on mixed lookup lines"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should require reuse or old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
