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
