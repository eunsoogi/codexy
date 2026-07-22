use serde_yaml::Value;
use std::{fs, path::Path};

use super::version_bump_pr_test_support::{
    has_trimmed_line, has_trimmed_line_start, trimmed_line_position,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn workflow_requires_issue_scope_and_reconciles_one_pr() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let document: Value = serde_yaml::from_str(&text)?;
    let workflow = document.as_mapping().ok_or("workflow root")?;
    let dispatch = workflow
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        .and_then(|(_, value)| value.get("workflow_dispatch"))
        .ok_or("workflow_dispatch")?;
    let issue = dispatch
        .get("inputs")
        .and_then(|inputs| inputs.get("issue"))
        .ok_or("governing issue input")?;
    assert_eq!(issue.get("required").and_then(Value::as_bool), Some(true));
    let permissions = workflow.get("permissions").ok_or("workflow permissions")?;
    assert_eq!(permissions.get("issues").and_then(Value::as_str), Some("write"));
    let steps = workflow
        .get("jobs")
        .and_then(|jobs| jobs.get("open-version-pr"))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .ok_or("version bump steps")?;
    let validate_issue = named_step_run(steps, "Validate governing release issue")?;
    let synchronize = named_step_run(steps, "Synchronize plugin version")?;
    let validate_release = named_step_run(steps, "Validate release candidate")?;
    let reconcile = named_step_run(steps, "Open version bump pull request")?;
    assert!(has_trimmed_line_start(validate_issue, "gh issue view "));
    assert!(has_trimmed_line_start(validate_issue, "scripts/render-version-pr-metadata "));
    assert_eq!(synchronize, "scripts/sync-plugin-version --version \"$VERSION\"");
    for command in [
        "scripts/sync-plugin-version --check",
        "scripts/validate-plugin-config --check",
        "cargo test --locked",
        "git diff --check",
    ] {
        assert!(has_trimmed_line(validate_release, command), "missing validation: {command}");
    }
    for start in [
        "gh pr list ",
        "if git ls-remote ",
        "gh api --method PUT ",
        "plugins/codexy/hooks/codexy-pr-title-check.sh ",
        "plugins/codexy/hooks/codexy-pr-label-check.sh ",
        "scripts/validate-plugin-config --check-completion-handoff ",
    ] {
        assert!(has_trimmed_line_start(reconcile, start), "missing reconciliation: {start}");
    }
    assert!(!reconcile.split_ascii_whitespace().any(|token| token == "--force"));
    assert!(trimmed_line_position(reconcile, "gh pr list ") < trimmed_line_position(reconcile, "git push "));
    assert!(has_trimmed_line(reconcile, "if [ \"$pr_count\" -eq 0 ] && [ \"$remote_exists\" = false ]; then"));
    assert!(has_trimmed_line(reconcile, "elif [ \"$remote_exists\" = true ]; then"));
    assert!(has_trimmed_line(
        reconcile,
        r#"git diff --binary --no-ext-diff origin/main..."origin/$branch" \"#,
    ));
    assert!(has_trimmed_line(reconcile, "if [ \"$pr_count\" -eq 0 ]; then"));
    assert!(has_trimmed_line(reconcile, "echo \"Branch and pull-request state disagree for ${branch}.\" >&2"));
    Ok(())
}

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
}
