use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/actions_external_finding_fixture.rs"]
mod actions_fixture;

const FULL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DELTA: &str = "cccccccccccccccccccccccccccccccccccccccc";
const CURRENT: &str = "dddddddddddddddddddddddddddddddddddddddd";

fn control() -> Value {
    let mut control = direct_state::post_cap_control_with_findings(
        actions_fixture::OWNING_ISSUE,
        FULL,
        DELTA,
        CURRENT,
        "authenticated_external_finding_repair",
        CURRENT,
        "PASS",
        json!([]),
        json!(["placeholder"]),
    );
    control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .expect("qualifying change")
        .remove("finding_ids");
    control
}

fn run_case<F>(name: &str, mutate: F) -> TestResult<()>
where
    F: FnOnce(&mut Value, &mut Value, &mut Value, &mut Value, &mut String),
{
    let temporary = tempfile::tempdir()?;
    let fixture = actions_fixture::ActionsGhFixture::write(temporary.path(), DELTA)?;
    let mut run = read_json(&fixture.run)?;
    let mut jobs = read_json(&fixture.jobs)?;
    let mut pulls = read_json(&fixture.pulls)?;
    let mut timeline = read_json(&fixture.timeline)?;
    let mut log = fs::read_to_string(&fixture.log)?;
    mutate(&mut run, &mut jobs, &mut pulls, &mut timeline, &mut log);
    write_json(&fixture.run, &run)?;
    write_json(&fixture.jobs, &jobs)?;
    write_json(&fixture.pulls, &pulls)?;
    write_json(&fixture.timeline, &timeline)?;
    fs::write(&fixture.log, log)?;

    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control(),
            "authenticated_actions_finding_locator": actions_fixture::locator()
        }))?,
    )?;
    let mut paths = vec![fixture.path.clone()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output)
        .env_path_list("PATH", paths)
        .env_path("ACTIONS_RUN", &fixture.run)
        .env_path("ACTIONS_JOBS", &fixture.jobs)
        .env_path("ACTIONS_PULLS", &fixture.pulls)
        .env_path("ACTIONS_TIMELINE", &fixture.timeline)
        .env_path("ACTIONS_LOG", &fixture.log);
    let result = producer.output()?;
    assert!(!result.status.success(), "negative case unexpectedly passed: {name}");
    Ok(())
}

fn read_json(path: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> TestResult<()> {
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}

#[test]
fn rejects_exact_identity_and_terminal_state_mutations() -> TestResult {
    run_case("wrong run", |run, _, _, _, _| run["id"] = json!(7002))?;
    run_case("wrong attempt", |run, jobs, _, _, _| {
        run["run_attempt"] = json!(2);
        jobs[0]["run_attempt"] = json!(2);
    })?;
    run_case("running workflow", |run, _, _, _, _| run["status"] = json!("in_progress"))?;
    run_case("successful workflow", |run, _, _, _, _| {
        run["conclusion"] = json!("success")
    })?;
    run_case("wrong job", |_, jobs, _, _, _| jobs[0]["id"] = json!(8002))?;
    run_case("running job", |_, jobs, _, _, _| jobs[0]["status"] = json!("in_progress"))?;
    run_case("successful job", |_, jobs, _, _, _| jobs[0]["conclusion"] = json!("success"))?;
    run_case("wrong step", |_, jobs, _, _, _| {
        jobs[0]["steps"][0]["name"] = json!("other step");
    })?;
    run_case("running step", |_, jobs, _, _, _| {
        jobs[0]["steps"][0]["status"] = json!("in_progress");
    })?;
    run_case("successful step", |_, jobs, _, _, _| {
        jobs[0]["steps"][0]["conclusion"] = json!("success");
    })?;
    run_case("wrong workflow", |run, _, _, _, _| {
        run["path"] = json!(".github/workflows/other.yml");
    })?;
    run_case("wrong pull", |_, _, pulls, _, _| pulls[0]["number"] = json!(99))?;
    run_case("wrong repository", |_, _, pulls, _, _| {
        pulls[0]["base"]["repo"]["full_name"] = json!("other/repository");
    })?;
    run_case("wrong issue", |_, _, _, timeline, _| {
        timeline[0]["source"]["issue"]["number"] = json!(99);
    })?;
    run_case("pull request source", |_, _, _, timeline, _| {
        timeline[0]["source"]["issue"]["pull_request"] = json!({
            "url": "https://api.github.com/repos/example/codexy-fixture/pulls/99"
        });
    })?;
    run_case("stale head", |run, _, _, _, _| {
        run["head_sha"] = json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    })?;
    run_case("invalid step window", |_, jobs, _, _, _| {
        jobs[0]["steps"][0]["completed_at"] = json!("2026-01-01T00:01:00Z");
    })?;
    Ok(())
}

#[test]
fn rejects_failure_outside_selected_step_window() -> TestResult {
    run_case("cross-step error", |_, _, _, _, log| {
        *log = "2026-01-01T00:00:59Z ERROR: outside.step (outside)
2026-01-01T00:00:60Z Traceback (most recent call last):
2026-01-01T00:00:61Z   File \"D:\\a\\codexy-fixture\\codexy-fixture\\packages\\getcodexy\\tests\\test_component_capability_probe.py\", line 57
2026-01-01T00:00:62Z NotImplementedError: outside
".into();
    })
}

#[test]
fn rejects_same_second_adjacent_step_failure() -> TestResult {
    run_case("adjacent step boundary", |_, jobs, _, _, log| {
        jobs[0]["steps"]
            .as_array_mut()
            .expect("steps")
            .push(json!({
                "number": 6,
                "name": "following step",
                "status": "completed",
                "conclusion": "failure",
                "started_at": "2026-01-01T00:02:00Z",
                "completed_at": "2026-01-01T00:03:00Z"
            }));
        *log = "2026-01-01T00:02:00.500Z ERROR: outside.step (outside)\n2026-01-01T00:02:00.600Z Traceback (most recent call last):\n2026-01-01T00:02:00.700Z   File \"D:\\a\\codexy-fixture\\codexy-fixture\\packages\\getcodexy\\tests\\test_component_capability_probe.py\", line 57\n2026-01-01T00:02:00.800Z NotImplementedError: outside\n".into();
    })
}

#[test]
fn legacy_locator_rejects_actions_fields() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control(),
            "authenticated_external_finding_locator": actions_fixture::locator()
        }))?,
    )?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output);
    assert!(!producer.output()?.status.success());
    Ok(())
}
