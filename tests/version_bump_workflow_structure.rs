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
    let checkout = steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some("Check out repository"))
        .ok_or("checkout step")?;
    assert_eq!(
        checkout.get("with").and_then(|with| with.get("fetch-depth")).and_then(Value::as_i64),
        Some(0),
    );
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
        "gh api --method GET \"repos/$GITHUB_REPOSITORY/pulls\" ",
        "if git ls-remote ",
        "gh api --method PUT ",
        "scripts/build-version-pr-state ",
        "scripts/plan-version-pr-reconciliation ",
        "plugins/codexy/hooks/codexy-pr-title-check.sh ",
        "plugins/codexy/hooks/codexy-pr-label-check.sh ",
        "scripts/validate-plugin-config --check-completion-handoff ",
    ] {
        assert!(has_trimmed_line_start(reconcile, start), "missing reconciliation: {start}");
    }
    assert!(!reconcile.split_ascii_whitespace().any(|token| token == "--force"));
    assert!(
        trimmed_line_position(reconcile, "gh api --method GET \"repos/$GITHUB_REPOSITORY/pulls\" ")
            < trimmed_line_position(reconcile, "git push ")
    );
    assert!(has_trimmed_line(
        reconcile,
        r#"-f state=open -f head="$owner:$branch" \"#,
    ));
    assert!(has_trimmed_line(reconcile, "--arg oid \"$remote_oid\" \\"));
    assert!(has_trimmed_line(
        reconcile,
        r#"'.[0] | .headRepository == $repository and .headLabel == $label and .headRefOid == $oid' \"#,
    ));
    assert!(has_trimmed_line(reconcile, "--expected-head-oid \"$expected_head_oid\" \\"));
    assert!(has_trimmed_line(
        reconcile,
        r#"--has-changes true --pr-count "$pr_count" --remote-exists "$remote_exists" \"#,
    ));
    assert!(has_trimmed_line(
        reconcile,
        r#"--pr-matches-origin "$pr_matches_origin")"#,
    ));
    assert!(has_trimmed_line(
        reconcile,
        r#"git diff --binary --no-ext-diff origin/main..."origin/$branch" \"#,
    ));
    assert!(has_trimmed_line(reconcile, "if [ \"$action\" = first-run ]; then"));
    assert!(has_trimmed_line(reconcile, "if [ \"$action\" = pushed-no-pr ]; then"));
    Ok(())
}

#[test]
fn workflow_refreshes_snapshots_and_finalizes_only_after_readiness() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let provisional = workflow
        .find("--merge-message-checked false)")
        .ok_or("provisional planner")?;
    let provisional_render = workflow
        .find("render_version_pr_metadata \"$publication_phase\"")
        .ok_or("provisional render")?;
    let label_gate = workflow.find("codexy-pr-label-check.sh").ok_or("label gate")?;
    let handoff_gate = workflow.find("--check-completion-handoff").ok_or("handoff gate")?;
    let merge_gate = workflow.find("codexy-merge-message-check.sh").ok_or("merge gate")?;
    let proven = workflow
        .rfind("--merge-message-checked true)")
        .ok_or("proven planner")?;
    let final_publish = workflow
        .rfind("publish_version_pr_metadata \"$publication_phase\"")
        .ok_or("final publication")?;
    let publish_helper = workflow.find("publish_version_pr_metadata() ").ok_or("publish helper")?;
    let label_mutation = workflow.find("repos/$GITHUB_REPOSITORY/issues/$pr_number/labels").ok_or("label mutation")?;
    let final_edit = workflow.rfind("--body-file \"$state_dir/metadata/body.md\"").ok_or("final body edit")?;
    let final_state = workflow
        .rfind("scripts/build-version-pr-state ")
        .ok_or("final rebuilt state")?;

    assert!(workflow.matches("refresh_version_pr_snapshot").count() >= 2);
    assert_eq!(workflow.matches("publish_version_pr_metadata \"$publication_phase\"").count(), 2);
    assert!(provisional < provisional_render);
    assert!(provisional_render < final_state);
    assert!(final_state < label_gate);
    assert!(label_gate < handoff_gate);
    assert!(handoff_gate < merge_gate);
    assert!(merge_gate < proven);
    assert!(proven < final_publish);
    assert!(publish_helper < label_mutation);
    assert!(label_mutation < final_edit);
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
