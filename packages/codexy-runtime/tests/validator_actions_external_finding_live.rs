use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

const BASE: &str = "b6101641ae78239cd377091c4681b498ec475dc5";
const FULL: &str = "d15385945015bd0c4c9ef33c3ac26b508f7209cf";
const DELTA: &str = "9f514eb6637f8d5ce9c5d299c7123f69d3596955";
const CURRENT: &str = "77711769e6b96a63949684529682d6c098a0225d";
const RUN_ID: u64 = 33985176201;
const FINDING_PATH: &str = "packages/getcodexy/tests/test_component_capability_probe.py";

fn locator() -> Value {
    json!({
        "repository": "eunsoogi/codexy",
        "owningIssue": 951,
        "pullRequest": 953,
        "workflowRun": RUN_ID,
        "runAttempt": 1,
        "job": 101357252541u64,
        "workflowPath": ".github/workflows/python-package.yml",
        "jobName": "github-activation-windows",
        "stepName": "Test dependency-aware GitHub activation"
    })
}

fn snapshot(head: &str, control: Option<Value>) -> Value {
    let mut value = json!({
        "repository": "eunsoogi/codexy",
        "number": 953,
        "baseRefName": "main",
        "baseRefOid": BASE,
        "headRefOid": head,
        "url": "https://github.com/eunsoogi/codexy/pull/953",
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "owningIssue": {
                "repository": "eunsoogi/codexy",
                "number": 951,
                "url": "https://github.com/eunsoogi/codexy/issues/951",
                "association": "closing-issue-reference"
            }
        }
    });
    if let Some(control) = control {
        value["reviewControl"] = control;
    }
    value
}

#[test]
#[ignore = "requires an authenticated live GitHub Actions read"]
fn actual_actions_failure_reaches_producer_and_build_refresh() -> TestResult {
    let repository = codexy_runtime::paths::repository_root();
    let mut control = direct_state::post_cap_control_with_findings(
        951,
        FULL,
        DELTA,
        CURRENT,
        "authenticated_external_finding_repair",
        CURRENT,
        "PASS",
        json!([]),
        json!(["placeholder"]),
    );
    let change = control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .ok_or("qualifying change")?;
    change.remove("finding_ids");
    let previous = direct_state::post_cap_prior(&control);
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_actions_finding_locator": locator(),
            "current_pr_state": snapshot(CURRENT, None),
            "previous_pr_state": snapshot(DELTA, Some(previous.clone()))
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .args(["--repository-root"])
        .arg(repository)
        .status()?;
    assert!(result.success(), "live Actions source must pass producer");
    let produced: Value = serde_json::from_slice(&fs::read(&output)?)?;
    let source = &produced["post_cap_re_review"]["qualifying_change"]["external_finding"];
    assert_eq!(source["observedCommit"], DELTA);
    assert_eq!(source["source"]["workflowRun"], RUN_ID);
    assert_eq!(source["findings"][0]["path"], FINDING_PATH);

    let current_path = temporary.path().join("current.json");
    let control_path = temporary.path().join("control.json");
    let previous_path = temporary.path().join("previous.json");
    let admitted_path = temporary.path().join("admitted.json");
    fs::write(&current_path, serde_json::to_vec(&snapshot(CURRENT, None))?)?;
    fs::write(&control_path, serde_json::to_vec(&produced)?)?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&snapshot(DELTA, Some(previous)))?,
    )?;
    let build = Command::new(repository.join("scripts/build-pr-state"))
        .args(["--repository-root"])
        .arg(repository)
        .args(["--base-pr-state-file"])
        .arg(&current_path)
        .args(["--review-control-state-file"])
        .arg(&control_path)
        .args(["--previous-pr-state-file"])
        .arg(&previous_path)
        .args(["--output"])
        .arg(&admitted_path)
        .env(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        )
        .status()?;
    assert!(build.success(), "build-pr-state must refresh live Actions source");
    Ok(())
}
