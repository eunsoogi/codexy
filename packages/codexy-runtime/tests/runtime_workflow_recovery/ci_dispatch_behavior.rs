use serde_json::json;

#[path = "ci_dispatch_fixture.rs"]
mod fixture;
use fixture::{Fixture, WORKFLOWS};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_result(output: std::process::Output, success: bool) {
    assert_eq!(
        output.status.success(),
        success,
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn activation_dispatch_reuses_success_and_waits_for_active_ci_without_duplicates() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.change("rust-test.yml", |runs| {
        runs[0]["event"] = json!("workflow_dispatch");
        runs[0]["status"] = json!("in_progress");
        runs[0]["conclusion"] = json!("");
    })?;
    assert_result(fixture.run()?, true);
    assert_result(fixture.run()?, true);
    assert!(fixture.dispatched()?.is_empty());
    Ok(())
}

#[test]
fn activation_dispatch_creates_missing_ci_once_and_reuses_retry() -> TestResult {
    let fixture = Fixture::new()?;
    for workflow in WORKFLOWS {
        fixture.change(workflow, |runs| *runs = json!([]))?;
    }
    assert_result(fixture.run()?, true);
    assert_eq!(fixture.dispatched()?, WORKFLOWS);
    assert_result(fixture.run()?, true);
    assert_eq!(fixture.dispatched()?, WORKFLOWS);
    Ok(())
}

#[test]
fn activation_dispatch_passes_public_predecessor_for_an_unpublished_target() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.change("python-package.yml", |runs| *runs = json!([]))?;
    assert_result(fixture.run()?, true);
    assert_eq!(fixture.dispatched()?, ["python-package.yml"]);
    assert_eq!(fixture.python_baselines()?, ["1.6.3"]);
    Ok(())
}

#[test]
fn activation_dispatch_stops_when_no_valid_prior_public_version_exists() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.set_public_release_tags(&["v1.7.0", "v1.7.0-rc.1", "latest"])?;
    fixture.change("python-package.yml", |runs| *runs = json!([]))?;
    let output = fixture.run()?;
    assert!(
        !output.status.success(),
        "activation dispatch unexpectedly continued without a prior public release"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("no published stable release precedes target v1.7.0"),
        "missing baseline must explain how to recover: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.dispatched()?.is_empty());
    Ok(())
}

#[test]
fn activation_dispatch_cannot_reuse_other_purposes_bases_heads_or_events() -> TestResult {
    for (workflow, field, value) in [
        (
            "rust-test.yml",
            "displayTitle",
            format!("Measurement {}", fixture::HEAD),
        ),
        (
            "touched-loc-gate.yml",
            "displayTitle",
            format!("CI {} base {}", fixture::HEAD, "c".repeat(40)),
        ),
        ("language-lint.yml", "headSha", "c".repeat(40)),
        ("python-package.yml", "event", "push".into()),
    ] {
        let fixture = Fixture::new()?;
        fixture.change(workflow, |runs| runs[0][field] = json!(value))?;
        assert_result(fixture.run()?, true);
        assert_eq!(fixture.dispatched()?, [workflow]);
    }
    Ok(())
}

#[test]
fn activation_dispatch_preserves_latest_failure_instead_of_older_success_or_rerun() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.change("rust-test.yml", |runs| {
        let mut failed = runs[0].clone();
        failed["databaseId"] = json!(999);
        failed["conclusion"] = json!("failure");
        runs.as_array_mut().unwrap().push(failed);
    })?;
    assert_result(fixture.run()?, false);
    assert!(fixture.dispatched()?.is_empty());
    Ok(())
}

#[test]
fn activation_dispatch_excludes_unstarted_pr_runs_but_never_accepts_action_required() -> TestResult
{
    let fixture = Fixture::new()?;
    for workflow in WORKFLOWS {
        fixture.change(workflow, |runs| {
            runs[0]["displayTitle"] = json!("feat(runtime): activate v1.7.0");
            runs[0]["conclusion"] = json!("action_required");
        })?;
    }
    assert_result(fixture.run()?, true);
    assert_eq!(fixture.dispatched()?, WORKFLOWS);
    let denied = Fixture::new()?;
    denied.change("rust-test.yml", |runs| {
        runs[0]["conclusion"] = json!("action_required")
    })?;
    assert_result(denied.run()?, false);
    assert!(denied.dispatched()?.is_empty());
    Ok(())
}
