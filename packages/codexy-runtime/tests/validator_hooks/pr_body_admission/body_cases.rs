use super::super::admission_runtime::TestResult;
use serde_json::json;
use std::process::Command;

pub(super) const VALID_BODY: &str = "## Summary\n\nValidate PR body admission.\n\n## Rationale\n\nKeep routes consistent.\n\n## Changed Areas\n\nShared body adapter.\n\n## Verification\n\nRun route tests.\n\n## Evidence\n\nLauncher output is observed.\n\n## Not Run\n\nNo invalid remote write.\n\n## Follow-ups\n\nParent owns review and merge.\n\nFixes #949";
pub(super) const MINIMAL_INVALID_BODY: &str = "note";

fn body_cases() -> Vec<(&'static str, String, bool)> {
    vec![
        ("valid", VALID_BODY.to_owned(), false),
        ("minimal", MINIMAL_INVALID_BODY.to_owned(), true),
        (
            "missing-section",
            VALID_BODY.replace("## Follow-ups\n\nParent owns review and merge.\n\n", ""),
            true,
        ),
        (
            "fenced-headings",
            format!("```text\n{VALID_BODY}\n```"),
            true,
        ),
        (
            "comment-headings",
            format!("<!--\n{VALID_BODY}\n-->"),
            true,
        ),
        (
            "multiple-closing",
            VALID_BODY.replace("Fixes #949", "Closes #948\nFixes #949"),
            true,
        ),
        (
            "noncanonical-final",
            VALID_BODY.replace("Fixes #949", "Refs #949"),
            true,
        ),
    ]
}

#[test]
fn common_body_cases_are_checked_in_one_python_process() -> TestResult {
    let root = super::super::admission_runtime::plugin_root();
    let cases = body_cases()
        .into_iter()
        .map(|(case_id, body, denied)| json!({"case_id":case_id,"body":body,"denied":denied}))
        .collect::<Vec<_>>();
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python)
        .arg("-c")
        .arg(
            "import json, sys\nfrom codexy_policy.body import valid_pull_request_body\ncases = json.loads(sys.argv[1])\nfor case in cases:\n    if valid_pull_request_body(case['body']) != (not case['denied']):\n        raise SystemExit(case['case_id'])\n",
        )
        .arg(serde_json::to_string(&cases)?)
        .env("PYTHONPATH", root.join("hooks"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()?;
    assert!(
        output.status.success(),
        "common body validator cases failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
