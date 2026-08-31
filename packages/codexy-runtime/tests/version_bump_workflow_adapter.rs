
use super::version_pr_workflow_fixture::{Scenario, WorkflowFixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn production_workflow_adapter_local_surface_matrix() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    assert!(
        root.join("scripts/reconcile-version-pr").is_file(),
        "production workflow adapter is missing"
    );
    let fixture = WorkflowFixture::new(root)?;
    for scenario in [Scenario::NewPr, Scenario::MatchingExisting, Scenario::StaleExisting] {
        fixture.prepare(scenario)?;
        let output = fixture.run()?;
        assert!(
            output.status.success(),
            "{scenario:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mutations = fixture.mutation_events()?;
        let create_count = mutations.iter().filter(|event| *event == "pr-create").count();
        let label_count = mutations.iter().filter(|event| *event == "label-put").count();
        let edit_count = mutations.iter().filter(|event| *event == "pr-edit").count();
        assert_eq!(
            create_count,
            usize::from(matches!(scenario, Scenario::NewPr)),
            "{scenario:?} PR creation count"
        );
        assert_eq!(label_count, 2, "{scenario:?} label mutation count");
        assert_eq!(
            edit_count,
            if matches!(scenario, Scenario::NewPr) { 2 } else { 3 },
            "{scenario:?} PR edit count"
        );
        assert_eq!(
            fixture.branch_push_count()?,
            usize::from(!matches!(scenario, Scenario::MatchingExisting)),
            "{scenario:?} branch push count"
        );
        if matches!(scenario, Scenario::StaleExisting) {
            let remote_patch = fixture.remote_patch()?;
            assert_eq!(
                std::fs::read(fixture.artifact("expected.patch"))?,
                remote_patch,
                "stale candidate was not replaced by the current patch"
            );
            assert!(!String::from_utf8(remote_patch)?.contains("component-manifest"));
        }
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
        let pr_state: serde_json::Value = serde_json::from_slice(&std::fs::read(fixture.artifact("pr-state.json"))?)?;
        assert_eq!(pr_state["number"], 999, "{scenario:?} PR identity");
    }

    fixture.prepare(Scenario::MismatchedIssue)?;
    let output = fixture.run()?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not match requested issue")
    );
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");

    fixture.prepare(Scenario::UnexpectedStalePath)?;
    let output = fixture.run()?;
    assert!(!output.status.success(), "unexpected stale path was accepted");
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside its recorded candidate inventory"));
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");

    fixture.prepare(Scenario::UnexpectedDeletedPath)?;
    let output = fixture.run()?;
    assert!(!output.status.success(), "unexpected deleted path was accepted");
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside its recorded candidate inventory"));
    assert_eq!(fixture.branch_push_count()?, 0);
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");

    fixture.prepare(Scenario::StaleExisting)?;
    fixture.install_remote_head_race()?;
    let output = fixture.run()?;
    assert!(!output.status.success(), "remote-head race was accepted");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("failed to push"),
        "race stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    Ok(())
}

#[test]
fn governing_issue_request_is_canonicalized_before_mutation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let fixture = WorkflowFixture::new(root)?;

    for request in ["301", "0301"] {
        fixture.prepare(Scenario::NewPr)?;
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
        assert!(!merge_gate.contains("--expected-issue"), "{request}");
    }

    for request in ["0", "not-a-number", "301;echo"] {
        fixture.prepare(Scenario::NewPr)?;
        let output = fixture.run_with_issue(request)?;
        assert!(!output.status.success(), "{request} was accepted");
        assert_eq!(fixture.mutation_events()?, Vec::<String>::new(), "{request}");
        assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    }

    fixture.prepare(Scenario::NewPr)?;
    let output = fixture.run_with_issue("302")?;
    assert!(!output.status.success(), "request/API mismatch was accepted");
    assert_eq!(fixture.mutation_events()?, Vec::<String>::new());
    assert_eq!(std::fs::read(fixture.mutation_sentinel())?, b"unchanged\n");
    Ok(())
}
