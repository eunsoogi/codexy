use std::fs;

use serde_json::json;

use crate::support::{FixtureCommand, TestResult};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_review_graph.rs"]
mod graph;
#[path = "support/actions_external_finding_fixture.rs"]
mod actions_fixture;

use actions_fixture::{
    ActionsGhFixture, FINDING_PATH, OWNING_ISSUE, RUN_ID, TARGET_ISSUE, locator,
};

#[test]
fn actions_source_reaches_producer_build_and_handoff() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create_with_external_path(
        temporary.path(),
        FINDING_PATH,
    )?;
    let raw_control = direct_state::post_cap_control_with_findings(
        TARGET_ISSUE,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        "authenticated_external_finding_repair",
        direct_state::SYNTHETIC_EXTERNAL_EVIDENCE,
        "PASS",
        json!([]),
        json!(["placeholder"]),
    );
    let (mut control, base, _) = repository.prepare(
        &raw_control,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .ok_or("qualifying change")?
        .remove("finding_ids");
    let delta = control["post_cap_re_review"]["qualifying_change"]["from_head"]
        .as_str()
        .ok_or("delta head")?
        .to_owned();
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let previous = direct_state::post_cap_prior(&control);
    let current = actions_fixture::snapshot(&base, current_head, None);
    let previous_state = actions_fixture::snapshot(&base, &delta, Some(previous));
    let fixture = ActionsGhFixture::write(temporary.path(), &delta)?;
    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_actions_finding_locator": locator(),
            "current_pr_state": current,
            "previous_pr_state": previous_state
        }))?,
    )?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    let mut paths = vec![fixture.path.clone()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output)
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .env_path_list("PATH", paths.clone())
        .env_path("ACTIONS_RUN", &fixture.run)
        .env_path("ACTIONS_JOBS", &fixture.jobs)
        .env_path("ACTIONS_PULLS", &fixture.pulls)
        .env_path("ACTIONS_TIMELINE", &fixture.timeline)
        .env_path("ACTIONS_SOURCE_OWNERSHIP", &fixture.source_ownership)
        .env_path("ACTIONS_LOG", &fixture.log);
    let result = producer.output()?;
    assert!(
        result.status.success(),
        "Actions producer must accept the authenticated locator: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let produced_control: serde_json::Value = serde_json::from_slice(&fs::read(&output)?)?;
    assert_eq!(
        produced_control["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["capture"]["method"],
        "actions"
    );
    assert_eq!(
        produced_control["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["observedCommit"],
        delta
    );
    assert_eq!(
        produced_control["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["findings"][0]["path"],
        FINDING_PATH
    );
    assert_eq!(
        produced_control["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["owningIssue"]["number"],
        OWNING_ISSUE
    );
    assert_eq!(
        current["capture"]["owningIssue"]["number"],
        TARGET_ISSUE
    );

    let current_path = temporary.path().join("current-pr-state.json");
    let control_path = temporary.path().join("review-control.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let admitted_path = temporary.path().join("admitted-pr-state.json");
    let mut current_state = current.clone();
    current_state["state"] = json!("OPEN");
    current_state["isDraft"] = json!(true);
    current_state["mergeStateStatus"] = json!("CLEAN");
    current_state["reviewProfile"] = json!("strict");
    fs::write(&current_path, serde_json::to_vec(&current_state)?)?;
    fs::write(&control_path, serde_json::to_vec(&produced_control)?)?;
    fs::write(&previous_path, serde_json::to_vec(&previous_state)?)?;
    let mut build = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    build
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .args(["--base-pr-state-file"])
        .arg_path(&current_path)
        .args(["--review-control-state-file"])
        .arg_path(&control_path)
        .args(["--previous-pr-state-file"])
        .arg_path(&previous_path)
        .args(["--output"])
        .arg_path(&admitted_path)
        .env_path("CODEXY_REVIEW_CONTROL_BIN", env!("CARGO_BIN_EXE_codexy-review-control"))
        .env_path_list("PATH", paths.clone())
        .env_path("ACTIONS_RUN", &fixture.run)
        .env_path("ACTIONS_JOBS", &fixture.jobs)
        .env_path("ACTIONS_PULLS", &fixture.pulls)
        .env_path("ACTIONS_TIMELINE", &fixture.timeline)
        .env_path("ACTIONS_SOURCE_OWNERSHIP", &fixture.source_ownership)
        .env_path("ACTIONS_LOG", &fixture.log);
    let built = build.output()?;
    assert!(
        built.status.success(),
        "build-pr-state must refresh the Actions source: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let admitted = fs::read(&admitted_path)?;
    let admitted: serde_json::Value = serde_json::from_slice(&admitted)?;
    assert_eq!(
        admitted["reviewControl"]["post_cap_re_review"]["qualifying_change"]
            ["external_finding"]["source"]["workflowRun"],
        RUN_ID
    );

    let handoff_path = temporary.path().join("completion-handoff.md");
    fs::write(&handoff_path, "PASS on the exact current head.\n")?;
    let mut handoff = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-validate"));
    handoff
        .args(["--check-completion-handoff", "--handoff-file"])
        .arg_path(&handoff_path)
        .args(["--pr-state-file"])
        .arg_path(&admitted_path)
        .env_path_list("PATH", paths)
        .env_path("ACTIONS_RUN", &fixture.run)
        .env_path("ACTIONS_JOBS", &fixture.jobs)
        .env_path("ACTIONS_PULLS", &fixture.pulls)
        .env_path("ACTIONS_TIMELINE", &fixture.timeline)
        .env_path("ACTIONS_SOURCE_OWNERSHIP", &fixture.source_ownership)
        .env_path("ACTIONS_LOG", &fixture.log);
    let handed_off = handoff.output()?;
    assert!(
        handed_off.status.success(),
        "completion handoff must refresh the Actions source: {}",
        String::from_utf8_lossy(&handed_off.stderr)
    );
    Ok(())
}

#[test]
fn actions_and_graphql_locators_are_mutually_exclusive() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": direct_state::strict_control(951, "synthetic-current-head"),
            "authenticated_external_finding_locator": {},
            "authenticated_actions_finding_locator": {}
        }))?,
    )?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output);
    let result = producer.output()?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("exactly one")
    );
    Ok(())
}

#[test]
fn caller_supplied_actions_source_is_rejected() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": direct_state::strict_control(951, "synthetic-current-head"),
            "authenticated_actions_finding": {"source": {"kind": "github-actions"}}
        }))?,
    )?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output);
    let result = producer.output()?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("caller-supplied"));
    Ok(())
}
