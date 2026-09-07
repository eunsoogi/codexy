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
pub(crate) const TARGET_ISSUE: u64 = 13;
pub(crate) const FINDING_PATH: &str = "packages/getcodexy/tests/test_component_capability_probe.py";
const STEP_STARTED: &str = "2026-01-01T00:01:00Z";
const STEP_COMPLETED: &str = "2026-01-01T00:02:00Z";

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
                "number": TARGET_ISSUE,
                "url": format!("https://github.com/{REPOSITORY}/issues/{TARGET_ISSUE}"),
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
    pub(crate) source_ownership: PathBuf,
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
        let source_ownership = root.join("source-ownership.json");
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
                    {"number": 5, "name": "synthetic failing unittest step", "status": "completed", "conclusion": "failure", "started_at": STEP_STARTED, "completed_at": STEP_COMPLETED}
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
                "event": "cross-referenced",
                "source": {
                    "type": "issue",
                    "issue": {
                        "number": OWNING_ISSUE,
                        "repository": {"full_name": REPOSITORY},
                        "pull_request": null
                    }
                }
            }]))?,
        )?;
        fs::write(
            &source_ownership,
            serde_json::to_vec(&json!({
                "data": {
                    "repository": {
                        "pullRequest": {
                            "number": PULL_REQUEST,
                            "url": format!("https://github.com/{REPOSITORY}/pull/{PULL_REQUEST}"),
                            "repository": {"nameWithOwner": REPOSITORY},
                            "closingIssuesReferences": {
                                "nodes": [{
                                    "number": OWNING_ISSUE,
                                    "url": format!("https://github.com/{REPOSITORY}/issues/{OWNING_ISSUE}"),
                                    "repository": {"nameWithOwner": REPOSITORY}
                                }],
                                "pageInfo": {"hasNextPage": false}
                            }
                        }
                    }
                }
            }))?,
        )?;
        fs::write(
            &log,
            "2026-01-01T00:01:01Z ERROR: packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests.test_process_result_captures_bounded_diagnostics (packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests)\n2026-01-01T00:01:02Z Traceback (most recent call last):\n2026-01-01T00:01:03Z   File \"D:\\a\\codexy-fixture\\codexy-fixture\\packages\\getcodexy\\tests\\test_component_capability_probe.py\", line 57, in test_process_result_captures_bounded_diagnostics\n2026-01-01T00:01:04Z   File \"C:\\python\\lib\\pathlib.py\", line 1195, in cwd\n2026-01-01T00:01:05Z   File \"C:\\python\\lib\\pathlib.py\", line 1223, in absolute\n2026-01-01T00:01:06Z NotImplementedError: cannot instantiate 'PosixPath' on your system\n",
        )?;
        let gh = bin.join("gh");
        fs::write(
            &gh,
            r##"#!/bin/sh
set -eu
case "$*" in
  "api --help") printf '%s\n' '  --allow-escape-sequences  Include terminal escape sequences in output' ;;
  *"/actions/runs/"*"/attempts/1/jobs?per_page=100"*) cat "$ACTIONS_JOBS" ;;
  *"/actions/runs/"*"/attempts/1"*) cat "$ACTIONS_RUN" ;;
  *"/commits/"*"/pulls?per_page=100"*) cat "$ACTIONS_PULLS" ;;
  *"/issues/"*"/timeline?per_page=100"*) cat "$ACTIONS_TIMELINE" ;;
  *"graphql"*) cat "$ACTIONS_SOURCE_OWNERSHIP" ;;
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
            source_ownership,
            log,
        })
    }

    pub(crate) fn write_rejecting_unsupported_escape_sequence(
        root: &std::path::Path,
        delta: &str,
    ) -> TestResult<Self> {
        let fixture = Self::write(root, delta)?;
        let gh = fixture.path.join("gh");
        let backend = fixture.path.join("gh.backend");
        fs::rename(&gh, &backend)?;
        fs::write(
            &gh,
            r##"#!/bin/sh
set -eu
case "$*" in
  "api --help") printf '%s\n' '  --hostname HOST  GitHub hostname' ; exit 0 ;;
esac
for argument in "$@"; do
  if test "$argument" = "--allow-escape-sequences"; then
    echo "unknown flag: --allow-escape-sequences" >&2
    exit 2
  fi
done
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$script_dir/gh.backend" "$@"
"##,
        )?;
        make_executable(&gh)?;
        Ok(fixture)
    }
}
