use std::fs;

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult, make_executable};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_disposition_fixture.rs"]
mod disposition_fixture;
#[path = "support/post_cap_review_graph.rs"]
mod graph;

#[test]
fn next_review_eligibility_accepts_an_authentic_two_event_predecessor() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let control = direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    );
    let (control, previous_base, current_base) =
        repository.prepare(&control, direct_state::SYNTHETIC_BASE, direct_state::SYNTHETIC_BASE)?;
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let previous_control = direct_state::post_cap_prior(&control);
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("previous head")?
        .to_owned();
    let current = direct_state::pr_snapshot(947, &current_base, current_head, None);
    let previous = direct_state::pr_snapshot(
        947,
        &previous_base,
        &previous_head,
        Some(previous_control),
    );
    let current_path = temporary.path().join("current.json");
    let previous_path = temporary.path().join("previous.json");
    let input_path = temporary.path().join("input.json");
    let output_path = temporary.path().join("eligibility.json");
    fs::write(&current_path, serde_json::to_vec(&current)?)?;
    fs::write(&previous_path, serde_json::to_vec(&previous)?)?;
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
    let (path, ci_response, maintainer_response) = fake_gh(
        temporary.path(),
        disposition_fixture::ci_response(947, &current_base, current_head),
        disposition_fixture::maintainer_response(947, 947, &current_base, current_head),
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
        .env_path_list("PATH", path)
        .env_path("CODEXY_TEST_CI_RESPONSE", ci_response)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", maintainer_response);
    let result = command.output()?;
    assert!(
        result.status.success(),
        "pre-verdict eligibility must accept the real two-event predecessor: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&fs::read(output_path)?)?;
    assert_eq!(
        receipt["schema"],
        "codexy.review-control-next-review-eligibility.v1"
    );
    assert_eq!(receipt["eligible"], true);
    assert_eq!(receipt["target"]["pullRequest"], 947);
    assert_eq!(receipt["target"]["headRefOid"], current_head);
    assert_eq!(receipt["predecessor"]["terminalReviewCount"], 2);
    assert_eq!(receipt["predecessor"]["delta"]["terminalResult"], "BLOCK");
    assert_eq!(receipt["predecessor"]["delta"]["reviewedHead"], previous_head);
    assert_eq!(receipt["coverage"].as_array().map(Vec::len), Some(3));
    assert_eq!(receipt["evidence"]["kind"], "authenticated_finding_disposition");
    assert!(receipt.get("reviewControl").is_none());
    Ok(())
}

#[test]
fn next_review_eligibility_rejects_caller_classification_and_existing_event() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let control = direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    );
    let (control, previous_base, current_base) =
        repository.prepare(&control, direct_state::SYNTHETIC_BASE, direct_state::SYNTHETIC_BASE)?;
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let previous_control = direct_state::post_cap_prior(&control);
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("previous head")?
        .to_owned();
    let current = direct_state::pr_snapshot(947, &current_base, current_head, None);
    let previous = direct_state::pr_snapshot(
        947,
        &previous_base,
        &previous_head,
        Some(previous_control),
    );
    let current_path = temporary.path().join("current.json");
    let previous_path = temporary.path().join("previous.json");
    let input_path = temporary.path().join("input.json");
    let output_path = temporary.path().join("eligibility.json");
    fs::write(&current_path, serde_json::to_vec(&current)?)?;
    fs::write(&previous_path, serde_json::to_vec(&previous)?)?;
    fs::write(
        &input_path,
        serde_json::to_vec(&json!({
            "authenticated": true,
            "finding_ids": ["forged"],
            "authenticated_finding_disposition_locator": {
                "repository": "eunsoogi/codexy",
                "owningIssue": 947,
                "pullRequest": 947,
                "maintainerComment": 5554573060u64
            }
        }))?,
    )?;
    let (path, ci_response, maintainer_response) = fake_gh(
        temporary.path(),
        disposition_fixture::ci_response(947, &current_base, current_head),
        disposition_fixture::maintainer_response(947, 947, &current_base, current_head),
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
        .env_path_list("PATH", path)
        .env_path("CODEXY_TEST_CI_RESPONSE", ci_response)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", maintainer_response);
    let result = command.output()?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown field"));

    let mut current_with_event = current;
    current_with_event["reviewControl"] = control;
    fs::write(&current_path, serde_json::to_vec(&current_with_event)?)?;
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
        .arg_path(&repository.path);
    let result = command.output()?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("must not carry reviewControl"));
    Ok(())
}

fn fake_gh(
    root: &std::path::Path,
    ci: Value,
    maintainer: Value,
) -> TestResult<(Vec<std::path::PathBuf>, std::path::PathBuf, std::path::PathBuf)> {
    let bin = root.join("bin");
    fs::create_dir(&bin)?;
    let ci_response = root.join("ci-response.json");
    let maintainer_response = root.join("maintainer-response.json");
    fs::write(&ci_response, serde_json::to_vec(&ci)?)?;
    fs::write(&maintainer_response, serde_json::to_vec(&maintainer)?)?;
    let gh = bin.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nif [ \"$1\" = \"pr\" ]; then cat \"$CODEXY_TEST_CI_RESPONSE\"; else cat \"$CODEXY_TEST_MAINTAINER_RESPONSE\"; fi\n",
    )?;
    make_executable(&gh)?;
    let mut path = vec![bin];
    if let Some(existing) = std::env::var_os("PATH") {
        path.extend(std::env::split_paths(&existing));
    }
    Ok((path, ci_response, maintainer_response))
}
