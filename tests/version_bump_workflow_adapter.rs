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
        let mutations = fixture.mutation_events()?;
        let expected = match scenario {
            Scenario::NewPr => {
                ["pr-create", "label-put", "pr-edit", "label-put", "pr-edit"]
            }
            Scenario::MatchingExisting => {
                ["pr-edit", "label-put", "pr-edit", "label-put", "pr-edit"]
            }
            Scenario::MismatchedIssue => unreachable!(),
        };
        assert_eq!(mutations, expected, "{scenario:?} mutation order");
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
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    Ok(())
}

#[test]
fn governing_issue_request_is_canonicalized_before_mutation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for request in ["301", "0301"] {
        let fixture = WorkflowFixture::new(root, Scenario::NewPr)?;
        let output = fixture.run_with_issue(request)?;
        assert!(
            output.status.success(),
            "{request}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let gates = fixture.gate_events()?;
        let merge_gate = gates
            .lines()
            .find(|line| line.starts_with("--check-merge-message "))
            .ok_or("merge-message gate")?;
        let arguments = merge_gate.split_ascii_whitespace().collect::<Vec<_>>();
        let issue_index = arguments
            .iter()
            .position(|argument| *argument == "--expected-issue")
            .ok_or("expected issue argument")?;
        assert_eq!(arguments.get(issue_index + 1), Some(&"301"), "{request}");
    }

    for request in ["0", "not-a-number", "301;echo"] {
        let fixture = WorkflowFixture::new(root, Scenario::NewPr)?;
        let output = fixture.run_with_issue(request)?;
        assert!(!output.status.success(), "{request} was accepted");
        assert_eq!(fixture.mutation_events()?, Vec::<String>::new(), "{request}");
        assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    }

    let fixture = WorkflowFixture::new(root, Scenario::NewPr)?;
    let output = fixture.run_with_issue("302")?;
    assert!(!output.status.success(), "request/API mismatch was accepted");
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    Ok(())
}
