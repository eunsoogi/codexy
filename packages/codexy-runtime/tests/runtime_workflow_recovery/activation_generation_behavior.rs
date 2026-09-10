use super::activation_retry_behavior::fixture::{Fixture, git, success};
use super::activation_retry_behavior::receipt_with_identity;
use std::fs;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn activation_generation_blocks_a_competing_open_owner_before_push() -> TestResult {
    let fixture = Fixture::new("competing")?;
    let before = fixture.remote_head()?;
    let output = fixture.run("competing")?;
    assert!(!output.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostics.contains("competing runtime activation pull request"));
    assert_eq!(fixture.remote_head()?, before);
    Ok(())
}

#[test]
fn activation_generation_ignores_a_fork_competing_pr() -> TestResult {
    let fixture = Fixture::new("fork-competing")?;
    success(fixture.run("fork-owner")?)?;
    assert_ne!(fixture.remote_head()?, fixture.main);
    Ok(())
}

#[test]
fn activation_generation_rejects_a_same_repository_wrong_base_pr() -> TestResult {
    let fixture = Fixture::new("wrong-base")?;
    let before = fixture.remote_head()?;
    let output = fixture.run("wrong-base")?;
    assert!(!output.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostics.contains("activation pull request targets non-main base: release"));
    assert_eq!(fixture.remote_head()?, before);
    Ok(())
}

#[test]
fn activation_generation_accepts_a_same_repository_main_pr() -> TestResult {
    let fixture = Fixture::new("valid")?;
    let before = fixture.remote_head()?;
    success(fixture.run("same-main")?)?;
    assert_ne!(fixture.remote_head()?, before);
    Ok(())
}

#[test]
fn activation_generation_distinguishes_receipt_attempts() -> TestResult {
    let fixture = Fixture::new("new")?;
    let source = git(&fixture.repo, &["rev-parse", "main"])?;
    let tree = git(&fixture.repo, &["rev-parse", "main^{tree}"])?;
    let first = tempfile::NamedTempFile::new()?;
    fs::write(
        first.path(),
        serde_json::to_vec(&receipt_with_identity(&source, &tree, 42, 1))?,
    )?;
    let second = tempfile::NamedTempFile::new()?;
    fs::write(
        second.path(),
        serde_json::to_vec(&receipt_with_identity(&source, &tree, 42, 2))?,
    )?;
    assert_ne!(
        fixture.select_branch("1.7.0", first.path())?,
        fixture.select_branch("1.7.0", second.path())?
    );
    Ok(())
}

#[test]
fn activation_generation_rejects_a_saturated_open_pr_inventory() -> TestResult {
    let fixture = Fixture::new("saturated")?;
    let source = git(&fixture.repo, &["rev-parse", "main"])?;
    let tree = git(&fixture.repo, &["rev-parse", "main^{tree}"])?;
    let receipt = tempfile::NamedTempFile::new()?;
    fs::write(
        receipt.path(),
        serde_json::to_vec(&receipt_with_identity(&source, &tree, 42, 1))?,
    )?;
    let error = fixture
        .select_branch("1.7.0", receipt.path())
        .expect_err("saturated PR inventory was accepted");
    assert!(error.to_string().contains("inventory is saturated"));
    Ok(())
}

#[test]
fn activation_generation_does_not_confuse_a_longer_version_prefix() -> TestResult {
    let fixture = Fixture::new("adjacent-version")?;
    let source = git(&fixture.repo, &["rev-parse", "main"])?;
    let tree = git(&fixture.repo, &["rev-parse", "main^{tree}"])?;
    let receipt = tempfile::NamedTempFile::new()?;
    fs::write(
        receipt.path(),
        serde_json::to_vec(&receipt_with_identity(&source, &tree, 42, 1))?,
    )?;
    assert_eq!(
        fixture.select_branch("1.7.1", receipt.path())?,
        "codexy/runtime-activation-v1.7.1-staging-42-1"
    );
    Ok(())
}

#[test]
fn activation_generation_uses_a_new_branch_after_deleted_legacy_activation() -> TestResult {
    let fixture = Fixture::new("merged-deleted")?;
    assert_eq!(fixture.pr_states(&fixture.legacy_branch)?, "MERGED");
    assert!(
        git(
            &fixture.repo,
            &[
                "ls-remote",
                "origin",
                &format!("refs/heads/{}", fixture.legacy_branch)
            ],
        )?
        .is_empty()
    );
    assert_ne!(fixture.branch, fixture.legacy_branch);
    success(fixture.run("new-generation")?)?;
    let head = fixture.remote_head()?;
    assert_ne!(head, fixture.main);
    assert!(!git(&fixture.repo, &["diff", "--name-only", "main", &head])?.is_empty());
    Ok(())
}

#[test]
fn activation_generation_preserves_a_retained_legacy_branch() -> TestResult {
    let fixture = Fixture::new("retained")?;
    assert_eq!(fixture.pr_states(&fixture.legacy_branch)?, "MERGED");
    let legacy_head = fixture.remote_branch_head(&fixture.legacy_branch)?;
    success(fixture.run("retained-generation")?)?;
    assert_eq!(
        fixture.remote_branch_head(&fixture.legacy_branch)?,
        legacy_head
    );
    assert_ne!(fixture.remote_head()?, legacy_head);
    Ok(())
}
