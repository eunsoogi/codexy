use std::fs;

use super::super::{document, steps};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn retired_bootstrap_workflow_has_no_public_artifact_proof_path() -> TestResult {
    let bootstrap = document("bootstrap-package.yml")?;
    let steps = steps(&bootstrap, "publish-bootstrap")?;
    assert_eq!(steps.len(), 1);
    assert_eq!(
        steps[0]["name"],
        "Reject bootstrap-first PyPI publication"
    );
    assert!(steps[0]["run"].as_str().is_some_and(|run| run.contains("exit 1")));
    let source = fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/bootstrap-package.yml"),
    )?;
    for forbidden in [
        "pypa/gh-action-pypi-publish",
        "python -m build",
        "pypi.org/pypi",
        "pypi.org/simple",
    ] {
        assert!(!source.contains(forbidden), "retired bootstrap path retains {forbidden}");
    }
    Ok(())
}
