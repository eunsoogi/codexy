use std::{fs, path::{Path, PathBuf}};

use serde_json::{Value, json};

use crate::support::{TestResult, make_executable};

pub(crate) struct CiSources {
    pub(crate) pull_request: Value,
    pub(crate) required_status_checks: Value,
    pub(crate) expected_check_runs: Value,
    pub(crate) check_suites: Value,
}

pub(crate) struct GhFixture {
    pub(crate) path: Vec<PathBuf>,
    pub(crate) ci: PathBuf,
    pub(crate) required: PathBuf,
    pub(crate) expected: PathBuf,
    pub(crate) suites: PathBuf,
    pub(crate) maintainer: PathBuf,
}

pub(crate) fn ci_sources(pull_request: u64, base: &str, head: &str) -> CiSources {
    CiSources {
        pull_request: pull_response(pull_request, base, head),
        required_status_checks: json!({"required_status_checks": null}),
        expected_check_runs: json!([{
            "total_count": 1,
            "check_runs": [{
                "id": 1,
                "head_sha": head,
                "name": "Rust",
                "status": "completed",
                "conclusion": "success",
                "app": {"id": 15368},
                "check_suite": {"id": 1}
            }]
        }]),
        check_suites: json!([{
            "total_count": 1,
            "check_suites": [{
                "id": 1,
                "head_sha": head,
                "status": "completed",
                "conclusion": "success",
                "app": {"id": 15368}
            }]
        }]),
    }
}

fn pull_response(pull_request: u64, base: &str, head: &str) -> Value {
    json!({
        "number": pull_request,
        "baseRefName": "main",
        "baseRefOid": base,
        "headRefName": "feature",
        "headRefOid": head,
        "statusCheckRollup": [{
            "__typename": "CheckRun",
            "completedAt": "2026-09-06T00:00:00Z",
            "conclusion": "SUCCESS",
            "detailsUrl": "https://github.com/eunsoogi/codexy/actions/runs/1",
            "name": "Rust",
            "startedAt": "2026-09-06T00:00:00Z",
            "status": "COMPLETED",
            "workflowName": "Rust tests"
        }]
    })
}

pub(crate) fn write_gh_fixture(
    root: &Path,
    sources: &CiSources,
    maintainer: &Value,
) -> TestResult<GhFixture> {
    let bin = root.join("bin");
    fs::create_dir(&bin)?;
    let ci = root.join("ci-response.json");
    let required = root.join("required-status-response.json");
    let expected = root.join("expected-check-runs-response.json");
    let suites = root.join("check-suites-response.json");
    let maintainer_path = root.join("maintainer-response.json");
    fs::write(&ci, serde_json::to_vec(&sources.pull_request)?)?;
    fs::write(&required, serde_json::to_vec(&sources.required_status_checks)?)?;
    fs::write(&expected, serde_json::to_vec(&sources.expected_check_runs)?)?;
    fs::write(&suites, serde_json::to_vec(&sources.check_suites)?)?;
    fs::write(&maintainer_path, serde_json::to_vec(maintainer)?)?;
    let gh = bin.join("gh");
    fs::write(
        &gh,
        r#"#!/bin/sh
case "$*" in
  *"check-runs?per_page=100"*) cat "$CODEXY_TEST_EXPECTED_CHECKS_RESPONSE" ;;
  *"check-suites?per_page=100"*) cat "$CODEXY_TEST_CHECK_SUITES_RESPONSE" ;;
  *"branches/"*"/protection"*) cat "$CODEXY_TEST_REQUIRED_STATUS_RESPONSE" ;;
  *"graphql"*) cat "$CODEXY_TEST_MAINTAINER_RESPONSE" ;;
  *"pr view"*) cat "$CODEXY_TEST_CI_RESPONSE" ;;
  *) echo "unexpected gh fixture invocation: $*" >&2; exit 1 ;;
esac
"#,
    )?;
    make_executable(&gh)?;
    let mut path = vec![bin];
    if let Some(existing) = std::env::var_os("PATH") {
        path.extend(std::env::split_paths(&existing));
    }
    Ok(GhFixture {
        path,
        ci,
        required,
        expected,
        suites,
        maintainer: maintainer_path,
    })
}

pub(crate) fn maintainer_response(
    pull_request: u64,
    issue: u64,
    base: &str,
    head: &str,
) -> Value {
    let repository = "eunsoogi/codexy";
    let pull_url = format!("https://github.com/{repository}/pull/{pull_request}");
    let issue_url = format!("https://github.com/{repository}/issues/{issue}");
    let comment_id = 5_554_573_060u64;
    let comment_url = format!("{pull_url}#issuecomment-{comment_id}");
    let body = format!(
        "## Maintainer disposition recorded by the release orchestrator\n\nThis records the maintainer's existing instruction in the release conversation: differences between the actually used specialist models and the planned 1.7.0 specialist routing are accepted for this milestone. The orchestrator is recording that instruction, not obtaining or inventing a new approval.\n\nScope of this disposition:\n- Repository: {repository}\n- Owning issue: #{issue}\n- Pull request: #{pull_request}\n- Base: {base}\n- Head: {head}\n- Finding: selected-reviewer-policy-mismatch\n- Finding path: plugins/codexy/agents/codexy-sentinel.toml\n- Accepted difference: the retained Sentinel's actual gpt-5.6-sol/xhigh execution may stand despite the planned newer model routing. Preserve the actual native reviewer identity, runtime model, verdicts and review count; do not relabel execution or repeat review solely for the model difference.\n\nThis disposition accepts only that model-policy difference for the bound review history. It does not accept code defects, waive CI or review findings, authorize merge, reset review counters, or authorize a fourth review. Exact-head CI and the remaining code/source-provenance repair must be independently established. Future source validation must reread this comment and verify its identity, repository authority and exact scope."
    );
    json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "number": pull_request,
                    "url": pull_url,
                    "baseRefOid": base,
                    "headRefOid": head,
                    "repository": {"nameWithOwner": repository},
                    "comments": {
                        "nodes": [{
                            "id": "IC_kwDOS6i-_88AAAABSxQPBA",
                            "databaseId": comment_id,
                            "url": comment_url,
                            "body": body,
                            "createdAt": "2026-09-05T20:28:23Z",
                            "updatedAt": "2026-09-05T20:28:23Z",
                            "author": {"login": "eunsoogi"},
                            "authorAssociation": "OWNER",
                            "isMinimized": false
                        }],
                        "pageInfo": {"hasNextPage": false}
                    }
                },
                "issue": {
                    "number": issue,
                    "url": issue_url,
                    "repository": {"nameWithOwner": repository}
                }
            }
        }
    })
}
