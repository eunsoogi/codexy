use serde_yaml::Value;
use std::fs;

use super::version_bump_pr_test_support::has_trimmed_line_start;
use super::version_bump_workflow_contract::validate_version_pr_publication;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn workflow_requires_issue_scope_and_reconciles_one_pr() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
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
    let admission_run = named_step_run(steps, "Admit candidate version preparation")?;
    let synchronize = named_step_run(steps, "Prepare candidate plugin version")?;
    let validate_release = named_step_run(steps, "Validate release candidate")?;
    let reconcile_path = named_step_run(steps, "Open version bump pull request")?;
    assert_eq!(reconcile_path, "scripts/reconcile-version-pr");
    let reconcile = fs::read_to_string(root.join(reconcile_path))?;
    assert!(has_trimmed_line_start(validate_issue, "gh issue view "));
    assert!(has_trimmed_line_start(validate_issue, "scripts/render_version_pr_metadata.py "));
    assert!(synchronize.contains("scripts/sync-plugin-version.sh --prepare-candidate"));
    assert!(admission_run.contains("scripts/sync-plugin-version.sh --admit-candidate"));
    let issue_index = steps
        .iter()
        .position(|step| step["name"] == "Validate governing release issue")
        .ok_or("issue validation")?;
    let admission_index = steps
        .iter()
        .position(|step| step["name"] == "Admit candidate version preparation")
        .ok_or("candidate admission")?;
    let mutation_index = steps
        .iter()
        .position(|step| step["name"] == "Prepare candidate plugin version")
        .ok_or("candidate mutation")?;
    assert!(admission_index < issue_index && issue_index < mutation_index);
    assert!(validate_release.contains("scripts/sync-plugin-version.sh --check-candidate"));
    assert!(!text.contains("publish-forward-bootstrap"));
    assert!(!text.contains("project-bootstrap-candidate"));
    for command in [
        "scripts/sync-plugin-version.sh --check-candidate",
        "scripts/validate-plugin-config.sh --check",
        "cargo test --manifest-path packages/codexy-runtime/Cargo.toml --locked",
        "git diff --check",
    ] {
        assert!(validate_release.contains(command), "missing validation: {command}");
    }
    for start in [
        "gh api --method GET",
        "gh api --method PUT",
        "scripts/build-version-pr-state",
        "scripts/plan-version-pr-reconciliation",
        "plugins/codexy-github/hooks/codexy-pr-title-check.sh",
        "plugins/codexy-github/hooks/codexy-pr-label-check.sh",
        "scripts/validate-plugin-config.sh --check-completion-handoff",
    ] {
        assert!(has_trimmed_line_start(&reconcile, start), "missing reconciliation: {start}");
    }
    assert!(reconcile.contains("git ls-remote"));
    assert!(!reconcile.split_ascii_whitespace().any(|token| token == "--force"));
    assert!(reconcile.contains("git push"));
    assert!(reconcile.contains("state=open"));
    assert!(reconcile.contains("--arg oid"));
    assert!(reconcile.contains("headRepository == $repository"));
    assert!(reconcile.contains("--expected-head-oid"));
    assert!(reconcile.contains("--has-changes true"));
    assert!(reconcile.contains("--pr-count"));
    assert!(reconcile.contains("--remote-exists"));
    assert!(reconcile.contains("--pr-matches-origin"));
    assert!(reconcile.contains("--version \"$VERSION\""));
    assert!(reconcile.contains("--issue-json \"$state_dir/issue.json\""));
    assert!(reconcile.contains("git diff --binary --no-ext-diff"));
    assert!(reconcile.contains("first-run"));
    assert!(reconcile.contains("pushed-no-pr"));
    Ok(())
}

#[test]
fn workflow_refreshes_snapshots_and_finalizes_only_after_readiness() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let adapter = fs::read_to_string(root.join("scripts/reconcile-version-pr"))?;
    validate_version_pr_publication(&workflow, &adapter).map_err(std::io::Error::other)?;
    Ok(())
}

#[test]
fn workflow_delegates_reconciliation_below_the_loc_boundary() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    assert!(
        workflow.lines().count() <= 250,
        "workflow has {} lines",
        workflow.lines().count()
    );
    let document: Value = serde_yaml::from_str(&workflow)?;
    let steps = document["jobs"]["open-version-pr"]["steps"]
        .as_sequence()
        .ok_or("version bump steps")?;
    assert_eq!(
        named_step_run(steps, "Open version bump pull request")?,
        "scripts/reconcile-version-pr"
    );
    assert!(root.join("scripts/reconcile-version-pr").is_file());
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
