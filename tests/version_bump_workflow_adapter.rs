use std::path::Path;

use super::version_pr_workflow_fixture::{Scenario, WorkflowFixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn production_workflow_adapter_local_surface_matrix() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        root.join("scripts/reconcile-version-pr").is_file(),
        "production workflow adapter is missing"
    );
    for scenario in [Scenario::NewPr, Scenario::MatchingExisting] {
        let fixture = WorkflowFixture::new(root, scenario)?;
        let output = fixture.run()?;
        assert!(
            output.status.success(),
            "{scenario:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mutations = fixture.mutations()?;
        match scenario {
            Scenario::NewPr => assert!(mutations.contains("pr-create")),
            Scenario::MatchingExisting => assert!(!mutations.contains("pr-create")),
            Scenario::MismatchedIssue => unreachable!(),
        }
        assert!(mutations.contains("pr-edit"));
        assert!(mutations.contains("label-put"));
        for artifact in [
            "metadata/body.md",
            "metadata/title.txt",
            "metadata/labels.json",
            "pr-state.json",
            "handoff.md",
            "merge-message.txt",
        ] {
            assert!(fixture.artifact(artifact).is_file(), "missing {artifact}");
        }
    }

    let fixture = WorkflowFixture::new(root, Scenario::MismatchedIssue)?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not match requested issue")
    );
    assert_eq!(fixture.mutations()?, "");
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    Ok(())
}
