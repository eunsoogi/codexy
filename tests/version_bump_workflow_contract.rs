use serde_yaml::Value;

use super::version_bump_workflow_model::validate_version_pr_adapter;

const JOB: &str = "open-version-pr";
const STEP: &str = "Open version bump pull request";
const ADAPTER: &str = "scripts/reconcile-version-pr";

pub(super) fn validate_version_pr_publication(
    workflow: &str,
    adapter: &str,
) -> Result<(), String> {
    let document: Value = serde_yaml::from_str(workflow).map_err(|error| error.to_string())?;
    let run = document
        .get("jobs")
        .and_then(|jobs| jobs.get(JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step.get("name").and_then(Value::as_str) == Some(STEP))
        })
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {JOB}/{STEP} run command"))?;
    if run != ADAPTER {
        return Err(format!("{JOB}/{STEP} must invoke {ADAPTER}"));
    }
    validate_version_pr_adapter(adapter)
}
