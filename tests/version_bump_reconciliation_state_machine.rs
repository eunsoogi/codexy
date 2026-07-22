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

fn plan(
    root: &Path,
    has_changes: bool,
    pr_count: u8,
    remote_exists: bool,
    pr_matches: bool,
    current_dir: &Path,
) -> std::io::Result<std::process::Output> {
    Command::new(root.join("scripts/plan-version-pr-reconciliation"))
        .args([
            "--has-changes", if has_changes { "true" } else { "false" },
            "--pr-count", &pr_count.to_string(),
            "--remote-exists", if remote_exists { "true" } else { "false" },
            "--pr-matches-origin", if pr_matches { "true" } else { "false" },
        ])
        .current_dir(current_dir)
        .output()
}
