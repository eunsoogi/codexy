use crate::support::FixtureCommand as Command;
use serde_json::{json, Value};
use std::{fs, path::Path};

const PYTHON_MATRIX: &str = r#"
import importlib.util
import importlib.machinery
import json
import pathlib
import sys
import tempfile

identity_path = pathlib.Path(sys.argv[1]).resolve()
planner_path = pathlib.Path(sys.argv[2]).resolve()
cases_path = pathlib.Path(sys.argv[3]).resolve()
sys.path.insert(0, str(identity_path.parent))
identity_spec = importlib.util.spec_from_file_location("version_pr_identity", identity_path)
identity = importlib.util.module_from_spec(identity_spec)
sys.modules["version_pr_identity"] = identity
identity_spec.loader.exec_module(identity)
planner_loader = importlib.machinery.SourceFileLoader(
    "plan_version_pr_reconciliation", str(planner_path)
)
planner_spec = importlib.util.spec_from_loader(
    "plan_version_pr_reconciliation", planner_loader
)
planner = importlib.util.module_from_spec(planner_spec)
planner_loader.exec_module(planner)
cases = json.loads(cases_path.read_text())
if len(cases) != 24:
    raise SystemExit(f"expected 24 lower-level rows, found {len(cases)}")

for case in cases:
    with tempfile.TemporaryDirectory(prefix="codexy-version-identity-") as directory:
        sentinel = pathlib.Path(directory) / "mutation-sentinel"
        sentinel.write_text("unchanged\n")
        try:
            if case["kind"] == "transition":
                action = planner.plan(
                    case["has_changes"], case["pr_count"],
                    case["remote_exists"], case["pr_matches_origin"]
                )
                if action != case["action"]:
                    raise AssertionError(f"{case['name']}: action={action!r}")
                identity.authorize_governing_identity(
                    action, case["version"], case["repository"],
                    case["requested_issue"], case["observed_pr"]
                )
                actual_stdout, actual_stderr = action, ""
            else:
                identity.authorize_governing_identity(
                    "existing-pr-update", "1.3.1", "eunsoogi/codexy",
                    case["requested_issue"], case["observed_pr"]
                )
                actual_stdout, actual_stderr = "", ""
            actual_success = True
        except ValueError as error:
            actual_success = False
            actual_stdout, actual_stderr = "", str(error)
        if actual_success != case["success"]:
            raise AssertionError(f"{case['name']}: success={actual_success}")
        if actual_success:
            if actual_stdout != case["stdout"] or actual_stderr != "":
                raise AssertionError(f"{case['name']}: output mismatch")
            sentinel.write_text("mutation authorized\n")
            expected_sentinel = "mutation authorized\n"
        else:
            if actual_stderr != case["stderr"] or actual_stdout != "":
                raise AssertionError(
                    f"{case['name']}: stderr={actual_stderr!r}, expected={case['stderr']!r}"
                )
            expected_sentinel = "unchanged\n"
        if sentinel.read_text() != expected_sentinel:
            raise AssertionError(f"{case['name']}: sentinel mutation mismatch")
print(f"validated {len(cases)} governing-identity rows")
"#;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cases = cases();
    let cases_path = tempfile::NamedTempFile::new()?;
    fs::write(cases_path.path(), serde_json::to_vec(&cases)?)?;
    let output = Command::new("python3")
        .args(["-c", PYTHON_MATRIX])
        .arg(root.join("scripts/version_pr_identity.py"))
        .arg(root.join("scripts/plan-version-pr-reconciliation"))
        .arg(cases_path.path())
        .output()?;
    assert!(
        output.status.success(),
        "stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "validated 24 governing-identity rows");
    Ok(())
}

fn transition_case(
    name: &str,
    action: &str,
    requested_issue: Value,
    observed_pr: Option<Value>,
    success: bool,
    stderr: &str,
) -> Value {
    json!({
        "kind": "transition",
        "name": name,
        "action": action,
        "has_changes": true,
        "pr_count": if action == "existing-pr-update" { 1 } else { 0 },
        "remote_exists": action != "first-run",
        "pr_matches_origin": action == "existing-pr-update",
        "version": "1.3.1",
        "repository": "eunsoogi/codexy",
        "requested_issue": requested_issue,
        "observed_pr": observed_pr,
        "success": success,
        "stdout": if success { action } else { "" },
        "stderr": stderr,
    })
}

fn body_case(name: &str, body: &str, success: bool, stderr: &str) -> Value {
    let canonical = super::reference(301, "https://github.com/eunsoogi/codexy/issues/301");
    transition_case(
        name,
        "existing-pr-update",
        json!({"number": 301, "url": "https://github.com/eunsoogi/codexy/issues/301"}),
        Some(super::observed(json!([canonical]), body)),
        success,
        stderr,
    )
}

fn cases() -> Vec<Value> {
    let canonical = super::reference(301, "https://github.com/eunsoogi/codexy/issues/301");
    let issue = |number| json!({"number": number, "url": format!("https://github.com/eunsoogi/codexy/issues/{number}")});
    let mut cases = vec![
        transition_case("missing closing reference", "existing-pr-update", issue(301), Some(super::observed(json!([]), "Fixes #301\n")), false, "existing PR must have exactly one canonical closing issue reference"),
        transition_case("ambiguous body references", "existing-pr-update", issue(301), Some(super::observed(json!([canonical.clone()]), "Fixes #301\nFixes #301\n")), false, "observed PR API and body must agree on exactly one governing issue"),
        transition_case("multiple governing references", "existing-pr-update", issue(301), Some(super::observed(json!([canonical.clone(), super::reference(302, "https://github.com/eunsoogi/codexy/issues/302")]), "Fixes #301\n")), false, "existing PR must have exactly one canonical closing issue reference"),
        transition_case("malformed noncanonical reference", "existing-pr-update", issue(301), Some(super::observed(json!([super::reference(301, "https://github.com/eunsoogi/codexy/pull/301")]), "Fixes #301\n")), false, "observed closing issue reference requires a canonical issue URL"),
    ];
    for (name, body) in [
        ("singular fix alias", "Fix #301\nFixes #301\n"),
        ("past fix alias", "Fixed #301\nFixes #301\n"),
        ("singular close alias", "Close #301\nFixes #301\n"),
        ("plural close alias", "Closes #301\nFixes #301\n"),
        ("past close alias", "Closed #301\nFixes #301\n"),
        ("singular resolve alias", "Resolve #301\nFixes #301\n"),
        ("plural resolve alias", "Resolves #301\nFixes #301\n"),
        ("past resolve alias", "Resolved #301\nFixes #301\n"),
        ("case variant", "fIxEs #301\nFixes #301\n"),
        ("colon alias", "Resolves: #301\nFixes #301\n"),
        ("numbered canonical reference", "1. Fixes #301\nFixes #301\n"),
        ("bulleted alias", "- Fixed #301\nFixes #301\n"),
        ("different issue", "Closes #302\nFixes #301\n"),
        ("multiple references", "Closes #302, resolves #303\nFixes #301\n"),
        ("qualified different", "Fixes another/repository#301\nFixes #301\n"),
        ("qualified duplicate", "Fixes eunsoogi/codexy#301\nFixes #301\n"),
    ] {
        cases.push(body_case(name, body, false, "observed PR API and body must agree on exactly one governing issue"));
    }
    cases.push(body_case("missing separator", "Fixes#301\nFixes #301\n", false, "observed PR body contains a malformed closing reference"));
    for (name, body) in [
        ("canonical final reference", "Fixes #301\n"),
        ("non-closing prose", "This fixes release readiness.\n\nFixes #301\n"),
        ("bare issue mention", "See #302 for follow-up.\n\nFixes #301\n"),
    ] {
        cases.push(body_case(name, body, true, ""));
    }
    assert_eq!(cases.len(), 24);
    cases
}
