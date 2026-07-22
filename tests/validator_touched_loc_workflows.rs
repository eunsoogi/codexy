use crate::support;

use support::touched_loc::{fixture, regular_lines, stderr, validate, write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn touched_loc_governs_workflow_yaml_only() -> TestResult {
    for path in [
        ".github/workflows/oversized.yml",
        ".github/workflows/oversized.yaml",
    ] {
        let repo = fixture(path, regular_lines(251))?;
        let output = validate(repo.path())?;
        assert!(!output.status.success(), "{path} escaped governance");
        assert!(stderr(&output).contains(&format!("{path} has 251 lines")));
    }
    for path in ["config/oversized.yml", "fixtures/oversized.yaml"] {
        let repo = fixture(path, regular_lines(251))?;
        let output = validate(repo.path())?;
        assert!(output.status.success(), "{path}: {}", stderr(&output));
    }
    let repo = fixture(".github/workflows/boundary.yml", regular_lines(250))?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "boundary: {}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_accepts_cohesive_workflow_script_extraction() -> TestResult {
    let baseline = format!(
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n{}",
        regular_lines(247)
    );
    let repo = fixture(".github/workflows/release.yml", baseline)?;
    write(
        repo.path(),
        ".github/workflows/release.yml",
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: scripts/reconcile-release\n",
    )?;
    write(repo.path(), "scripts/reconcile-release", &regular_lines(247))?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_parses_only_safe_single_script_commands() -> TestResult {
    for command in [
        "scripts/reconcile-release --check",
        "scripts/reconcile-release --mode=check release",
        "scripts/reconcile-release --check --tag \"$RELEASE_TAG\"",
        "scripts/reconcile-release --tag $RELEASE_TAG",
        "scripts/reconcile-release --tag ${RELEASE_TAG}",
        "scripts/reconcile-release --tag \"${RELEASE_TAG}\"",
    ] {
        assert_workflow_extraction(command, true)?;
    }
    for command in [
        "command scripts/reconcile-release --check",
        "MODE=check scripts/reconcile-release",
        "scripts/reconcile-release > result.txt",
        "scripts/reconcile-release $(echo --check)",
        "scripts/reconcile-release `echo --check`",
        "scripts/reconcile-release $",
        "scripts/reconcile-release ${}",
        "scripts/reconcile-release ${RELEASE-TAG}",
        "scripts/reconcile-release $9",
        "scripts/reconcile-release \"$RELEASE_TAG",
        "scripts/reconcile-release prefix$RELEASE_TAG",
        "scripts/reconcile-release $RELEASE_TAG/suffix",
        "scripts/reconcile-release ${RELEASE_TAG:-latest}",
        "scripts/reconcile-release *.tgz",
        "scripts/reconcile-release | tee result.txt",
        "scripts/reconcile-release|tee result.txt",
        "scripts/reconcile-release && echo done",
        "scripts/reconcile-release&&echo done",
        "scripts/reconcile-release; echo done",
        "scripts/reconcile-release;echo done",
        "/scripts/reconcile-release --check",
        "scripts/../reconcile-release --check",
        "cargo run --bin reconcile-release",
    ] {
        assert_workflow_extraction(command, false)?;
    }
    Ok(())
}

fn assert_workflow_extraction(command: &str, accepted: bool) -> TestResult {
    let baseline = format!(
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n{}",
        regular_lines(247)
    );
    let repo = fixture(".github/workflows/release.yml", baseline)?;
    write(
        repo.path(),
        ".github/workflows/release.yml",
        &format!("name: fixture\njobs:\n  release:\n    steps:\n      - run: {command}\n"),
    )?;
    write(repo.path(), "scripts/reconcile-release", &regular_lines(247))?;
    let output = validate(repo.path())?;
    assert_eq!(output.status.success(), accepted, "{command}: {}", stderr(&output));
    Ok(())
}
