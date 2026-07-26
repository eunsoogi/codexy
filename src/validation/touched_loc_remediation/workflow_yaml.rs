use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct Workflow {
    #[serde(default)]
    jobs: BTreeMap<String, WorkflowJob>,
}

#[derive(Deserialize)]
struct WorkflowJob {
    #[serde(default)]
    steps: Vec<WorkflowStep>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WorkflowStep {
    Mapping(RunStep),
    Other(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
struct RunStep {
    #[serde(default)]
    run: Option<String>,
}

pub(super) fn run_commands(workflow: &str) -> Vec<String> {
    let Ok(workflow) = serde_yaml::from_str::<Workflow>(workflow) else {
        return Vec::new();
    };
    workflow
        .jobs
        .into_values()
        .flat_map(|job| job.steps)
        .filter_map(|step| match step {
            WorkflowStep::Mapping(step) => step.run,
            WorkflowStep::Other(_) => None,
        })
        .collect()
}
