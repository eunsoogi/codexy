use std::{fs, path::PathBuf};

use serde_json::{Value, json};

use crate::support::{TestResult, make_executable};

pub(crate) const REPOSITORY: &str = "example/codexy-fixture";
pub(crate) const RUN_ID: u64 = 7001;
pub(crate) const RUN_ATTEMPT: u64 = 1;
pub(crate) const JOB_ID: u64 = 8001;
pub(crate) const WORKFLOW_ID: u64 = 9001;
pub(crate) const PULL_REQUEST: u64 = 12;
pub(crate) const OWNING_ISSUE: u64 = 11;
pub(crate) const FINDING_PATH: &str = "packages/getcodexy/tests/test_component_capability_probe.py";

pub(crate) fn locator() -> serde_json::Value {
    json!({
        "repository": REPOSITORY,
        "owningIssue": OWNING_ISSUE,
        "pullRequest": PULL_REQUEST,
        "workflowRun": RUN_ID,
        "runAttempt": RUN_ATTEMPT,
        "job": JOB_ID,
        "workflowPath": ".github/workflows/synthetic-actions.yml",
        "jobName": "synthetic-failure-job",
        "stepName": "synthetic failing unittest step"
    })
}

pub(crate) fn snapshot(base: &str, head: &str, control: Option<Value>) -> Value {
    let mut value = json!({
        "repository": REPOSITORY,
        "number": PULL_REQUEST,
        "baseRefName": "main",
        "baseRefOid": base,
        "headRefOid": head,
        "url": format!("https://github.com/{REPOSITORY}/pull/{PULL_REQUEST}"),
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "owningIssue": {
                "repository": REPOSITORY,
                "number": OWNING_ISSUE,
                "url": format!("https://github.com/{REPOSITORY}/issues/{OWNING_ISSUE}"),
                "association": "linked-issue-reference"
            }
        }
    });
    if let Some(control) = control {
        value["reviewControl"] = control;
    }
    value
}

pub(crate) struct ActionsGhFixture {
    pub(crate) path: PathBuf,
    pub(crate) run: PathBuf,
    pub(crate) jobs: PathBuf,
    pub(crate) pulls: PathBuf,
    pub(crate) timeline: PathBuf,
    pub(crate) log: PathBuf,
}

impl ActionsGhFixture {
    pub(crate) fn write(root: &std::path::Path, delta: &str) -> TestResult<Self> {
        let bin = root.join("bin");
        fs::create_dir(&bin)?;
        let run = root.join("run.json");
        let jobs = root.join("jobs.json");
        let pulls = root.join("pulls.json");
        let timeline = root.join("timeline.json");
        let log = root.join("job.log");
        fs::write(
            &run,
            serde_json::to_vec(&json!({
                "id": RUN_ID, "run_attempt": RUN_ATTEMPT, "event": "pull_request",
                "workflow_id": WORKFLOW_ID, "path": ".github/workflows/synthetic-actions.yml",
                "head_sha": delta, "head_branch": "synthetic-actions-branch",
                "status": "completed", "conclusion": "failure"
            }))?,
        )?;
        fs::write(
            &jobs,
            serde_json::to_vec(&json!([{
                "id": JOB_ID, "name": "synthetic-failure-job", "run_attempt": RUN_ATTEMPT,
                "head_sha": delta, "status": "completed", "conclusion": "failure",
                "steps": [
                    {"number": 5, "name": "synthetic failing unittest step", "status": "completed", "conclusion": "failure"}
                ]
            }]))?,
        )?;
        fs::write(
            &pulls,
            serde_json::to_vec(&json!([{
                "number": PULL_REQUEST, "repository": REPOSITORY,
                "base": {"repo": {"full_name": REPOSITORY}},
                "head": {"sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
                "head_ref": "synthetic-actions-branch", "base_ref": "main"
            }]))?,
        )?;
        fs::write(
            &timeline,
            serde_json::to_vec(&json!([{
                "event": "cross-referenced", "source_number": OWNING_ISSUE,
                "source_repository": REPOSITORY
            }]))?,
        )?;
        fs::write(
            &log,
            "ERROR: packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests.test_process_result_captures_bounded_diagnostics (packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests)\nTraceback (most recent call last):\n  File \"D:\\a\\codexy-fixture\\codexy-fixture\\packages\\getcodexy\\tests\\test_component_capability_probe.py\", line 57, in test_process_result_captures_bounded_diagnostics\n  File \"C:\\python\\lib\\pathlib.py\", line 1195, in cwd\n  File \"C:\\python\\lib\\pathlib.py\", line 1223, in absolute\nNotImplementedError: cannot instantiate 'PosixPath' on your system\n",
        )?;
        let gh = bin.join("gh");
        fs::write(
            &gh,
            r##"#!/bin/sh
set -eu
case "$*" in
  *"/actions/runs/"*"/attempts/1/jobs?per_page=100"*) cat "$ACTIONS_JOBS" ;;
  *"/actions/runs/"*"/attempts/1"*) cat "$ACTIONS_RUN" ;;
  *"/commits/"*"/pulls?per_page=100"*) cat "$ACTIONS_PULLS" ;;
  *"/issues/"*"/timeline?per_page=100"*) cat "$ACTIONS_TIMELINE" ;;
  *"/actions/jobs/"*"/logs"*) cat "$ACTIONS_LOG" ;;
  *) echo "unexpected gh request: $*" >&2; exit 1 ;;
esac
"##,
        )?;
        make_executable(&gh)?;
        Ok(Self {
            path: bin,
            run,
            jobs,
            pulls,
            timeline,
            log,
        })
    }
}
