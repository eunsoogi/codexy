use std::fs;

use crate::support::{FixtureCommand, TestResult, make_executable};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_disposition_fixture.rs"]
mod disposition_fixture;
#[path = "support/post_cap_review_graph.rs"]
mod graph;

#[test]
fn completion_handoff_refreshes_current_and_changed_live_disposition_sources() -> TestResult {
    let control = disposition_control();
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, _, current_base) = repository.prepare(
        &control,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let current_head = control["reviewed_head"]
        .as_str()
        .ok_or("current head")?
        .to_owned();
    let issue = control["issue_number"].as_u64().ok_or("issue number")?;
    let mut state = direct_state::pr_snapshot(issue, &current_base, &current_head, None);
    state["state"] = serde_json::json!("OPEN");
    state["isDraft"] = serde_json::json!(true);
    state["mergeStateStatus"] = serde_json::json!("CLEAN");
    state["reviewProfile"] = control["profile"].clone();
    state["reviewControl"] = control;
    let state_path = temporary.path().join("pr-state.json");
    let handoff_path = temporary.path().join("handoff.md");
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    fs::write(&handoff_path, "PASS on the exact current head.\n")?;

    let ci_path = temporary.path().join("ci-response.json");
    let maintainer_path = temporary.path().join("maintainer-response.json");
    fs::write(
        &ci_path,
        serde_json::to_vec(&disposition_fixture::ci_response(
            issue,
            &current_base,
            &current_head,
        ))?,
    )?;
    fs::write(
        &maintainer_path,
        serde_json::to_vec(&disposition_fixture::maintainer_response(
            issue,
            issue,
            &current_base,
            &current_head,
        ))?,
    )?;
    let current = run_completion_handoff(
        &temporary,
        &handoff_path,
        &state_path,
        &ci_path,
        &maintainer_path,
    )?;
    assert!(
        current.status.success(),
        "completion handoff must accept current live sources: {}",
        String::from_utf8_lossy(&current.stderr)
    );

    let mut changed_ci = disposition_fixture::ci_response(issue, &current_base, &current_head);
    changed_ci["headRefOid"] = serde_json::json!("0000000000000000000000000000000000000000");
    fs::write(&ci_path, serde_json::to_vec(&changed_ci)?)?;
    let changed = run_completion_handoff(
        &temporary,
        &handoff_path,
        &state_path,
        &ci_path,
        &maintainer_path,
    )?;
    assert!(!changed.status.success(), "changed live CI source must be rejected");
    assert!(
        String::from_utf8_lossy(&changed.stderr).contains("sources disagree")
            || String::from_utf8_lossy(&changed.stderr).contains("stale"),
        "changed source diagnostic must be explicit: {}",
        String::from_utf8_lossy(&changed.stderr)
    );
    Ok(())
}

fn run_completion_handoff(
    temporary: &tempfile::TempDir,
    handoff: &std::path::Path,
    state: &std::path::Path,
    ci_response: &std::path::Path,
    maintainer_response: &std::path::Path,
) -> TestResult<std::process::Output> {
    let bin = temporary.path().join("bin");
    if !bin.exists() {
        fs::create_dir(&bin)?;
        let gh = bin.join("gh");
        fs::write(
            &gh,
            "#!/bin/sh\nif [ \"$1\" = \"pr\" ]; then cat \"$CODEXY_TEST_CI_RESPONSE\"; else cat \"$CODEXY_TEST_MAINTAINER_RESPONSE\"; fi\n",
        )?;
        make_executable(&gh)?;
    }
    let mut paths = vec![bin];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-validate"));
    command
        .args(["--check-completion-handoff", "--handoff-file"])
        .arg_path(handoff)
        .args(["--pr-state-file"])
        .arg_path(state)
        .args(["--plugin-root"])
        .arg_path(codexy_runtime::paths::repository_root().join("plugins/codexy"))
        .env_path_list("PATH", paths)
        .env_path("CODEXY_TEST_CI_RESPONSE", ci_response)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", maintainer_response);
    Ok(command.output()?)
}

fn disposition_control() -> serde_json::Value {
    direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    )
}
