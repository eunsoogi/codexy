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

#[test]
fn activation_post_push_pr_readback_accepts_bounded_metadata_delay() -> TestResult {
    let fixture = Fixture::new("readback-delay")?;
    let old = fixture.remote_head()?;
    let output = success(fixture.run("delayed")?)?;
    assert_ne!(fixture.remote_head()?, old);
    assert!(output.contains("PR readback attempt=3"), "{output}");
    Ok(())
}

#[test]
fn activation_post_push_pr_readback_rejects_wrong_missing_or_duplicate_prs() -> TestResult {
    for mode in ["readback-wrong", "readback-missing", "readback-duplicate"] {
        let fixture = Fixture::new(mode)?;
        let old = fixture.remote_head()?;
        let output = fixture.run("rejected-readback")?;
        assert!(!output.status.success(), "{mode}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains("expected="), "{mode}: {text}");
        assert!(
            text.contains(if mode == "readback-duplicate" {
                "duplicate activation PRs"
            } else {
                "activation PR readback exhausted"
            }),
            "{mode}: {text}"
        );
        assert_eq!(
            text.matches("PR readback attempt=").count(),
            if mode == "readback-duplicate" { 1 } else { 6 }
        );
        assert_ne!(fixture.remote_head()?, old, "push did not happen");
    }
    Ok(())
}

#[test]
fn activation_readback_does_not_retry_or_hide_a_rejected_push() -> TestResult {
    let fixture = Fixture::new("push-failure")?;
    let old = fixture.remote_head()?;
    let output = fixture.run("push-rejected")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("activation branch push failed"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PR readback attempt="));
    assert_eq!(fixture.remote_head()?, old);
    Ok(())
}
