use std::{fs, process::Output};

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_disposition_fixture.rs"]
mod disposition_fixture;
#[path = "support/post_cap_review_graph.rs"]
mod graph;

fn disposition_control() -> Value {
    direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    )
}

fn add_import_marker(control: &mut Value) {
    let history = control["terminal_review_history"].as_array().expect("history");
    control["pre_pr_import"] = json!({
        "schema": "codexy.review-control-pre-pr-history.v1",
        "source": {"provider":"codex_app","method":"read_thread","authenticated":true,"host_id":"synthetic"},
        "issue": {"repository":"eunsoogi/codexy","number":947,"url":"https://github.com/eunsoogi/codexy/issues/947"},
        "complete": true,
        "events": history.iter().enumerate().map(|(index, event)| json!({
            "id": event["id"],
            "thread_id": "synthetic-thread",
            "turn_id": format!("synthetic-turn-{index}"),
            "ordinal": index + 1
        })).collect::<Vec<_>>()
    });
}

fn run_eligibility(
    temporary: &tempfile::TempDir,
    repository: &graph::SyntheticRepository,
    current: &Value,
    previous: &Value,
    source_head: &str,
) -> TestResult<(Output, Option<Value>)> {
    let current_path = temporary.path().join("current.json");
    let previous_path = temporary.path().join("previous.json");
    let input_path = temporary.path().join("input.json");
    let output_path = temporary.path().join("eligibility.json");
    fs::write(&current_path, serde_json::to_vec(current)?)?;
    fs::write(&previous_path, serde_json::to_vec(previous)?)?;
    fs::write(
        &input_path,
        serde_json::to_vec(&json!({
            "authenticated_finding_disposition_locator": {
                "repository": "eunsoogi/codexy",
                "owningIssue": 947,
                "pullRequest": 947,
                "maintainerComment": 5554573060u64
            }
        }))?,
    )?;
    let fixture = disposition_fixture::write_gh_fixture(
        temporary.path(),
        &disposition_fixture::ci_sources(
            947,
            current["baseRefOid"].as_str().expect("base"),
            source_head,
        ),
        &disposition_fixture::maintainer_response(
            947,
            947,
            current["baseRefOid"].as_str().expect("base"),
            source_head,
        ),
    )?;
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    command
        .args(["--check-next-review-eligibility", "--input"])
        .arg_path(&input_path)
        .args(["--current-pr-state-file"])
        .arg_path(&current_path)
        .args(["--previous-pr-state-file"])
        .arg_path(&previous_path)
        .args(["--output"])
        .arg_path(&output_path)
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .env_path_list("PATH", fixture.path)
        .env_path("CODEXY_TEST_CI_RESPONSE", fixture.ci)
        .env_path("CODEXY_TEST_REQUIRED_STATUS_RESPONSE", fixture.required)
        .env_path("CODEXY_TEST_EXPECTED_CHECKS_RESPONSE", fixture.expected)
        .env_path("CODEXY_TEST_CHECK_SUITES_RESPONSE", fixture.suites)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", fixture.maintainer);
    let result = command.output()?;
    let receipt = result
        .status
        .success()
        .then(|| fs::read(&output_path))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, receipt))
}

#[test]
fn next_review_eligibility_uses_the_delta_head_when_snapshot_head_is_stale() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) = repository.prepare(
        &disposition_control(),
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let previous_control = direct_state::post_cap_prior(&control);
    let delta_head = previous_control["reviewed_head"]
        .as_str()
        .expect("delta head")
        .to_owned();
    let stale_snapshot_head = repository.resolve(
        direct_state::SYNTHETIC_FULL_HEAD,
        false,
        false,
        true,
    )?;
    let current_head = control["reviewed_head"].as_str().expect("current head");
    let current = direct_state::pr_snapshot(947, &current_base, current_head, None);
    let mut previous = direct_state::pr_snapshot(
        947,
        &previous_base,
        &delta_head,
        Some(previous_control.clone()),
    );
    previous["headRefOid"] = Value::String(stale_snapshot_head);
    let mut previous_control = previous["reviewControl"].clone();
    add_import_marker(&mut previous_control);
    previous["reviewControl"] = previous_control;
    let (result, receipt) = run_eligibility(
        &temporary,
        &repository,
        &current,
        &previous,
        current_head,
    )?;
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(receipt.expect("receipt")["predecessor"]["delta"]["reviewedHead"], delta_head);
    Ok(())
}

#[test]
fn next_review_eligibility_rejects_broken_delta_to_current_ancestry() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) = repository.prepare(
        &disposition_control(),
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let previous_control = direct_state::post_cap_prior(&control);
    let delta_head = previous_control["reviewed_head"]
        .as_str()
        .expect("delta head")
        .to_owned();
    let broken_head = repository.resolve(
        direct_state::SYNTHETIC_FULL_HEAD,
        false,
        false,
        true,
    )?;
    let current = direct_state::pr_snapshot(947, &current_base, &broken_head, None);
    let previous = direct_state::pr_snapshot(
        947,
        &previous_base,
        &delta_head,
        Some(previous_control),
    );
    let (result, _) = run_eligibility(&temporary, &repository, &current, &previous, &broken_head)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("not ordered"));
    Ok(())
}
