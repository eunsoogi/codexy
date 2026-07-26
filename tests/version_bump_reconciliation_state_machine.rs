use std::{fs, path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn reconciliation_plan_covers_all_mutation_boundaries() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;
    let sentinel = temp.path().join("sentinel");
    fs::write(&sentinel, b"unchanged\n")?;

    for (name, has_changes, pr_count, remote_exists, pr_matches, expected) in [
        ("no-op", false, 0, false, false, "no-op"),
        ("first-run", true, 0, false, false, "first-run"),
        ("pushed-no-pr", true, 0, true, false, "pushed-no-pr"),
        ("existing-pr-update", true, 1, true, true, "existing-pr-update"),
    ] {
        let output = plan(root, has_changes, pr_count, remote_exists, pr_matches, temp.path())?;
        assert!(output.status.success(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8(output.stdout)?.trim(), expected, "{name}");
        assert_eq!(fs::read(&sentinel)?, b"unchanged\n", "{name} mutated state");
    }
    Ok(())
}

#[test]
fn reconciliation_mismatch_fails_before_mutation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, pr_count, remote_exists, expected_error) in [
        ("stale-pr-head", 1, true, "does not match"),
        ("duplicate-prs", 2, true, "more than one"),
        ("pr-without-branch", 1, false, "state disagree"),
    ] {
        let temp = tempfile::tempdir()?;
        let sentinel = temp.path().join("sentinel");
        fs::write(&sentinel, b"unchanged\n")?;
        let output = plan(root, true, pr_count, remote_exists, false, temp.path())?;
        assert!(!output.status.success(), "{name}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected_error), "{name}");
        assert_eq!(fs::read(&sentinel)?, b"unchanged\n", "{name}");
    }
    Ok(())
}

#[test]
fn existing_update_without_observed_identity_fails_before_authorization() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;
    let sentinel = temp.path().join("sentinel");
    fs::write(&sentinel, b"unchanged\n")?;
    let output = plan_without_identity(root, true, 1, true, true, temp.path())?;
    assert!(
        !output.status.success(),
        "branch/head equality authorized an identity-less existing PR update"
    );
    assert_eq!(fs::read(&sentinel)?, b"unchanged\n");
    Ok(())
}

#[test]
fn publication_phase_advances_only_after_every_readiness_gate() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, release, labels, handoff, merge_message, expected) in [
        ("release-failure", false, false, false, false, None),
        ("provisional", true, false, false, false, Some("provisional")),
        ("label-only", true, true, false, false, None),
        ("handoff-without-merge", true, true, true, false, None),
        ("proven", true, true, true, true, Some("proven")),
    ] {
        let output = Command::new(root.join("scripts/plan-version-pr-reconciliation"))
            .args([
                "--publication-phase",
                "--release-candidate-passed",
                if release { "true" } else { "false" },
                "--labels-checked",
                if labels { "true" } else { "false" },
                "--completion-handoff-checked",
                if handoff { "true" } else { "false" },
                "--merge-message-checked",
                if merge_message { "true" } else { "false" },
            ])
            .output()?;
        match expected {
            Some(phase) => {
                assert!(output.status.success(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
                assert_eq!(String::from_utf8(output.stdout)?.trim(), phase, "{name}");
            }
            None => assert!(!output.status.success(), "{name} advanced prematurely"),
        }
    }
    Ok(())
}

fn plan(
    root: &Path,
    has_changes: bool,
    pr_count: u8,
    remote_exists: bool,
    pr_matches: bool,
    current_dir: &Path,
) -> std::io::Result<std::process::Output> {
    let issue = current_dir.join("plan-issue.json");
    let observed = current_dir.join("plan-observed-pr.json");
    fs::write(
        &issue,
        br#"{"number":301,"url":"https://github.com/eunsoogi/codexy/issues/301"}"#,
    )?;
    fs::write(
        &observed,
        br#"{"headRefName":"codexy/version-1.3.1","body":"Fixes #301\n","labels":[{"name":"status/review"}],"closingIssuesReferences":[{"number":301,"url":"https://github.com/eunsoogi/codexy/issues/301","repository":{"name":"codexy","owner":{"login":"eunsoogi"}}}]}"#,
    )?;
    let mut command = planner(root, has_changes, pr_count, remote_exists, pr_matches);
    if has_changes {
        command
            .args(["--version", "1.3.1", "--repository", "eunsoogi/codexy"])
            .arg("--issue-json")
            .arg(&issue);
        if pr_count == 1 {
            command.arg("--observed-pr-json").arg(&observed);
        }
    }
    command.current_dir(current_dir).output()
}

fn plan_without_identity(
    root: &Path,
    has_changes: bool,
    pr_count: u8,
    remote_exists: bool,
    pr_matches: bool,
    current_dir: &Path,
) -> std::io::Result<std::process::Output> {
    planner(root, has_changes, pr_count, remote_exists, pr_matches)
        .current_dir(current_dir)
        .output()
}

fn planner(
    root: &Path,
    has_changes: bool,
    pr_count: u8,
    remote_exists: bool,
    pr_matches: bool,
) -> Command {
    let mut command = Command::new(root.join("scripts/plan-version-pr-reconciliation"));
    command.args([
        "--has-changes", if has_changes { "true" } else { "false" },
        "--pr-count", &pr_count.to_string(),
        "--remote-exists", if remote_exists { "true" } else { "false" },
        "--pr-matches-origin", if pr_matches { "true" } else { "false" },
    ]);
    command
}
