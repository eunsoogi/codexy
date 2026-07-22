use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn governing_identity_transition_matrix() -> TestResult {
    let canonical = reference(301, "https://github.com/eunsoogi/codexy/issues/301");
    let cases = [
        (
            "matching existing PR",
            "existing-pr-update",
            301,
            Some(observed(json!([canonical.clone()]), "Fixes #301\n")),
            true,
            "",
        ),
        (
            "different requested issue",
            "existing-pr-update",
            302,
            Some(observed(json!([canonical.clone()]), "Fixes #301\n")),
            false,
            "does not match requested issue",
        ),
        (
            "missing closing reference",
            "existing-pr-update",
            301,
            Some(observed(json!([]), "Fixes #301\n")),
            false,
            "exactly one canonical closing issue",
        ),
        (
            "ambiguous body references",
            "existing-pr-update",
            301,
            Some(observed(json!([canonical.clone()]), "Fixes #301\nFixes #301\n")),
            false,
            "body must end with exactly one canonical closing issue",
        ),
        (
            "multiple governing references",
            "existing-pr-update",
            301,
            Some(observed(
                json!([
                    canonical.clone(),
                    reference(302, "https://github.com/eunsoogi/codexy/issues/302")
                ]),
                "Fixes #301\n",
            )),
            false,
            "exactly one canonical closing issue",
        ),
        (
            "malformed noncanonical reference",
            "existing-pr-update",
            301,
            Some(observed(
                json!([reference(
                    301,
                    "https://github.com/eunsoogi/codexy/pull/301"
                )]),
                "Fixes #301\n",
            )),
            false,
            "canonical issue URL",
        ),
        ("new branch and PR", "first-run", 301, None, true, ""),
        ("existing branch without PR", "pushed-no-pr", 301, None, true, ""),
    ];

    for (name, action, requested_issue, observed_pr, expected, error) in cases {
        let temp = tempfile::tempdir()?;
        let issue = temp.path().join("issue.json");
        let observed_path = temp.path().join("observed-pr.json");
        let sentinel = temp.path().join("mutation-sentinel");
        fs::write(
            &issue,
            serde_json::to_vec(&json!({
                "number": requested_issue,
                "url": format!("https://github.com/eunsoogi/codexy/issues/{requested_issue}")
            }))?,
        )?;
        if let Some(value) = observed_pr {
            fs::write(&observed_path, serde_json::to_vec(&value)?)?;
        }
        fs::write(&sentinel, b"unchanged\n")?;

        let output = transition(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            action,
            &issue,
            observed_path.exists().then_some(observed_path.as_path()),
        )?;
        if output.status.success() {
            fs::write(&sentinel, b"mutation authorized\n")?;
        }
        assert_eq!(
            output.status.success(),
            expected,
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if expected {
            assert_eq!(String::from_utf8(output.stdout)?.trim(), action, "{name}");
            assert_eq!(fs::read(&sentinel)?, b"mutation authorized\n", "{name}");
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).contains(error), "{name}");
            assert_eq!(fs::read(&sentinel)?, b"unchanged\n", "{name} mutated");
        }
    }
    Ok(())
}

fn reference(number: u64, url: &str) -> Value {
    json!({
        "number": number,
        "url": url,
        "repository": {"name": "codexy", "owner": {"login": "eunsoogi"}}
    })
}

fn observed(references: Value, body: &str) -> Value {
    json!({
        "number": 999,
        "headRefName": "codexy/version-1.3.1",
        "body": body,
        "labels": [{"name": "status/review"}],
        "closingIssuesReferences": references
    })
}

fn transition(
    root: &Path,
    action: &str,
    issue: &Path,
    observed: Option<&Path>,
) -> std::io::Result<std::process::Output> {
    let (pr_count, remote_exists, pr_matches) = match action {
        "first-run" => ("0", "false", "false"),
        "pushed-no-pr" => ("0", "true", "false"),
        "existing-pr-update" => ("1", "true", "true"),
        _ => unreachable!(),
    };
    let mut command = Command::new(root.join("scripts/plan-version-pr-reconciliation"));
    command.args([
        "--has-changes", "true", "--pr-count", pr_count,
        "--remote-exists", remote_exists, "--pr-matches-origin", pr_matches,
        "--version", "1.3.1", "--repository", "eunsoogi/codexy",
    ]);
    command.arg("--issue-json").arg(issue);
    if let Some(path) = observed {
        command.arg("--observed-pr-json").arg(path);
    }
    command.output()
}
