use std::{fs, path::Path};

use serde_json::Value;

use crate::support::{FixtureCommand, TestResult, make_executable};
#[path = "review_control_direct_state.rs"]
mod direct_state;
#[path = "post_cap_review_graph.rs"]
mod graph;
#[path = "post_cap_external_finding_fixture.rs"]
mod external_finding_fixture;
#[path = "post_cap_disposition_fixture.rs"]
mod disposition_fixture;
#[path = "post_cap_review/disposition.rs"]
mod disposition;

#[allow(unused_imports)]
pub(crate) use disposition::{produce_disposition, run_build_with_disposition_maintainer, validate_readiness};

pub(crate) fn build_pr_state(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) =
        repository.prepare(control, previous_base, current_base)?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        &control,
        &previous_base,
        &current_base,
    )?;
    let result = invoke_build(
        &repository.path,
        &current,
        &control_path,
        &previous,
        &output,
        &control,
        None,
    )?;
    assert!(
        result.status.success(),
        "build-pr-state must accept the typed post-cap state: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}

pub(crate) fn run_build(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) =
        repository.prepare(control, previous_base, current_base)?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        &control,
        &previous_base,
        &current_base,
    )?;
    Ok(invoke_build(
        &repository.path,
        &current,
        &control_path,
        &previous,
        &output,
        &control,
        None,
    )?)
}

fn write_review_inputs(
    root: &std::path::Path,
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<(
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
)> {
    let issue_number = control["issue_number"].as_u64().ok_or("issue number")?;
    let previous = direct_state::post_cap_prior(control);
    let previous_head = previous["reviewed_head"]
        .as_str()
        .ok_or("prior head")?
        .to_owned();
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let current = root.join("current-pr-state.json");
    let control_path = root.join("review-control.json");
    let previous_path = root.join("previous-pr-state.json");
    fs::write(
        &current,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            current_base,
            current_head,
            None,
        ))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            previous_base,
            &previous_head,
            Some(previous),
        ))?,
    )?;
    Ok((current, control_path, previous_path))
}
fn invoke_build(
    repository: &Path,
    current: &std::path::Path,
    control_path: &std::path::Path,
    previous: &std::path::Path,
    output: &std::path::Path,
    review_control: &Value,
    disposition_sources: Option<(Value, Value)>,
) -> TestResult<std::process::Output> {
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--repository-root")
        .arg_path(repository)
        .arg("--base-pr-state-file")
        .arg_path(current)
        .arg("--review-control-state-file")
        .arg_path(control_path)
        .arg("--previous-pr-state-file")
        .arg_path(previous)
        .arg("--output")
        .arg_path(output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    #[cfg(unix)]
    if review_control["post_cap_re_review"]["reason"].as_str()
        == Some("authenticated_external_finding_repair")
    {
        let bin = output.parent().ok_or("build output parent")?.join("bin");
        fs::create_dir(&bin)?;
        let response = external_finding_fixture::pr938_response(
            review_control["post_cap_re_review"]["qualifying_change"]["from_head"]
                .as_str()
                .ok_or("external finding prior head")?,
        );
        let response_file = output.parent().ok_or("build output parent")?.join("github-response.json");
        fs::write(&response_file, serde_json::to_vec(&response)?)?;
        let gh = bin.join("gh");
        fs::write(&gh, "#!/bin/sh\ncat \"$CODEXY_TEST_GITHUB_RESPONSE\"\n")?;
        make_executable(&gh)?;
        let mut paths = vec![bin];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        command
            .env_path_list("PATH", paths)
            .env_path("CODEXY_TEST_GITHUB_RESPONSE", response_file);
    } else if review_control["post_cap_re_review"]["reason"].as_str()
        == Some("authenticated_finding_disposition")
    {
        let bin = output.parent().ok_or("build output parent")?.join("bin");
        fs::create_dir(&bin)?;
        let state: Value = serde_json::from_slice(&fs::read(current)?)?;
        let base = state["baseRefOid"].as_str().ok_or("disposition base")?;
        let head = state["headRefOid"].as_str().ok_or("disposition head")?;
        let issue = review_control["issue_number"].as_u64().ok_or("disposition issue")?;
        let pull = state["number"].as_u64().ok_or("disposition pull")?;
        let ci_response = output.parent().ok_or("build output parent")?.join("ci-response.json");
        let maintainer_response = output.parent().ok_or("build output parent")?.join("maintainer-response.json");
        let (ci_value, maintainer_value) = disposition_sources.unwrap_or_else(|| {
            (
                disposition_fixture::ci_response(pull, base, head),
                disposition_fixture::maintainer_response(pull, issue, base, head),
            )
        });
        fs::write(&ci_response, serde_json::to_vec(&ci_value)?)?;
        fs::write(&maintainer_response, serde_json::to_vec(&maintainer_value)?)?;
        let gh = bin.join("gh");
        fs::write(&gh, "#!/bin/sh\nif [ \"$1\" = \"pr\" ]; then cat \"$CODEXY_TEST_CI_RESPONSE\"; else cat \"$CODEXY_TEST_MAINTAINER_RESPONSE\"; fi\n")?;
        make_executable(&gh)?;
        let mut paths = vec![bin];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        command
            .env_path_list("PATH", paths)
            .env_path("CODEXY_TEST_CI_RESPONSE", ci_response)
            .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", maintainer_response);
    }
    Ok(command.output()?)
}
