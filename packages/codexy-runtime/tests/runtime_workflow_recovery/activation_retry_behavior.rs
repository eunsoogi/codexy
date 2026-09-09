use std::fs;
#[path = "activation_retry_fixture.rs"]
mod fixture;
#[path = "activation_retry_receipt.rs"]
mod receipt;
use fixture::{Fixture, git, success};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn activation_retry_integrates_advanced_main_preserving_artifact_and_same_pr() -> TestResult {
    let fixture = Fixture::new("valid")?;
    let old_head = fixture.remote_head()?;
    let artifact = git(
        &fixture.repo,
        &[
            "show",
            &format!("{old_head}:.agents/plugins/runtime-activation.json"),
        ],
    )?;
    success(fixture.run("first")?)?;
    let integrated = fixture.remote_head()?;
    assert_ne!(integrated, old_head);
    assert_eq!(
        fs::read_to_string(fixture.repo.join("retry-main-marker.txt"))?,
        "new main contract\n"
    );
    assert_eq!(
        git(
            &fixture.repo,
            &["show", "HEAD:.agents/plugins/runtime-activation.json"]
        )?,
        artifact
    );
    git(
        &fixture.repo,
        &["merge-base", "--is-ancestor", &old_head, &integrated],
    )?;
    git(
        &fixture.repo,
        &["merge-base", "--is-ancestor", "main", &integrated],
    )?;
    fixture.prepare_next_attempt()?;
    success(fixture.run("second")?)?;
    assert_eq!(
        fixture.remote_head()?,
        integrated,
        "retry created an unnecessary commit"
    );
    Ok(())
}

#[test]
fn activation_retry_rejects_unexpected_changes_provenance_and_merge_conflicts() -> TestResult {
    for mutation in ["unexpected", "provenance", "conflict"] {
        let fixture = Fixture::new(mutation)?;
        let old_head = fixture.remote_head()?;
        let output = fixture.run("rejected")?;
        assert!(!output.status.success(), "accepted {mutation}");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !diagnostics.contains("stale-branch-verifier"),
            "executed branch-owned verifier for {mutation}"
        );
        assert!(
            diagnostics.contains(if mutation == "conflict" {
                "CONFLICT"
            } else {
                "activation branch differs from verified contract"
            }),
            "{mutation}: {diagnostics}"
        );
        assert_eq!(
            fixture.remote_head()?,
            old_head,
            "changed remote on {mutation}"
        );
    }
    Ok(())
}

#[test]
fn activation_retry_rejects_changed_input_tree_and_unrelated_staging_source() -> TestResult {
    for mutation in ["late-index", "source"] {
        let fixture = Fixture::new(mutation)?;
        let before = fixture.remote_head()?;
        let output = fixture.run("rejected")?;
        assert!(!output.status.success(), "accepted {mutation}");
        if mutation == "late-index" {
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("activation input tree changed after verification")
            );
        }
        assert_eq!(fixture.remote_head()?, before);
    }
    Ok(())
}

#[test]
fn activation_new_branch_uses_the_same_tree_contract_and_reuses_its_pr() -> TestResult {
    let fixture = Fixture::new("new")?;
    success(fixture.run("first")?)?;
    let head = fixture.remote_head()?;
    fixture.prepare_next_attempt()?;
    success(fixture.run("second")?)?;
    assert_eq!(fixture.remote_head()?, head);
    Ok(())
}

#[test]
fn activation_verifier_step_authentication_is_scoped_and_required() -> TestResult {
    for step in [
        "Apply verified activation and version-selection contract",
        "Stage and verify activation branch",
    ] {
        let fixture = Fixture::new("valid")?;
        let before = fixture.remote_head()?;
        let output = fixture.run_with_omitted_token("missing-token", Some(step))?;
        assert_eq!(output.status.code(), Some(4), "{step}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("verifier GitHub query requires GH_TOKEN"),
            "{step}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(step));
        assert_eq!(
            fixture.remote_head()?,
            before,
            "{step} changed remote state"
        );
    }
    Ok(())
}
