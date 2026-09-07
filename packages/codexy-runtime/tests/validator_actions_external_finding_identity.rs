use std::fs;

use serde_json::json;

use crate::support::{FixtureCommand, TestResult};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_review_graph.rs"]
mod graph;
#[path = "support/actions_external_finding_fixture.rs"]
mod actions_fixture;

use actions_fixture::{ActionsGhFixture, FINDING_PATH, TARGET_ISSUE, locator};

#[test]
fn actions_rejects_jointly_changed_locator_and_timeline() -> TestResult {
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
    let mut timeline: serde_json::Value = serde_json::from_slice(&fs::read(&fixture.timeline)?)?;
    timeline[0]["source"]["issue"]["number"] = json!(TARGET_ISSUE);
    fs::write(&fixture.timeline, serde_json::to_vec(&timeline)?)?;
    let mut locator = locator();
    locator["owningIssue"] = json!(TARGET_ISSUE);

    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_actions_finding_locator": locator,
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
        .env_path_list("PATH", paths)
        .env_path("ACTIONS_RUN", &fixture.run)
        .env_path("ACTIONS_JOBS", &fixture.jobs)
        .env_path("ACTIONS_PULLS", &fixture.pulls)
        .env_path("ACTIONS_TIMELINE", &fixture.timeline)
        .env_path("ACTIONS_SOURCE_OWNERSHIP", &fixture.source_ownership)
        .env_path("ACTIONS_LOG", &fixture.log);
    let result = producer.output()?;
    assert!(
        !result.status.success(),
        "jointly changed Actions locator and timeline must be rejected: {}",
        String::from_utf8_lossy(&result.stdout)
    );
    Ok(())
}
