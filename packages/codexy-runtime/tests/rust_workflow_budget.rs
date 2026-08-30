use serde_yaml::{Mapping, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const REQUIRED_JOBS: [(&str, &str); 2] = [
    ("rust-test", "Ubuntu"),
    ("windows-rust-test", "Windows"),
];

#[test]
fn rust_workflow_has_one_fail_closed_five_minute_suite_per_platform() -> TestResult {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/rust-test.yml"),
    )?;
    let document: Value = serde_yaml::from_str(&workflow)?;
    let jobs = mapping_field(document.as_mapping(), "jobs", "workflow")?;
    let mut failures = Vec::new();

    for (job_id, platform) in REQUIRED_JOBS {
        let job = mapping_field(Some(jobs), job_id, "jobs")?;
        if job.get("timeout-minutes").and_then(Value::as_u64) != Some(5) {
            failures.push(format!("{platform} job is missing timeout-minutes: 5"));
        }
        let full_suites = cargo_test_steps(job)
            .filter(|run| run.contains("--all-targets"))
            .collect::<Vec<_>>();
        if full_suites.len() != 1 {
            failures.push(format!(
                "{platform} job has {} equivalent all-targets workloads; expected 1",
                full_suites.len()
            ));
        } else if !full_suites[0].contains("--locked") {
            failures.push(format!("{platform} all-targets workload is not locked"));
        }
        if weakens_failure_propagation(job) {
            failures.push(format!("{platform} job weakens command failure propagation"));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

fn mapping_field<'a>(
    mapping: Option<&'a Mapping>,
    key: &str,
    context: &str,
) -> Result<&'a Mapping, Box<dyn std::error::Error>> {
    mapping
        .and_then(|mapping| mapping.get(key))
        .and_then(Value::as_mapping)
        .ok_or_else(|| format!("{context} missing mapping {key}").into())
}

fn cargo_test_steps(job: &Mapping) -> impl Iterator<Item = &str> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_mapping)
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .filter(|run| run.contains("cargo test"))
}

fn weakens_failure_propagation(job: &Mapping) -> bool {
    if job.get("continue-on-error").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    job.get("steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_mapping)
        .any(|step| {
            let run = step.get("run").and_then(Value::as_str).unwrap_or_default();
            let pwsh_test = step.get("shell").and_then(Value::as_str) == Some("pwsh")
                && run.contains("cargo test");
            step.get("continue-on-error").and_then(Value::as_bool) == Some(true)
                || run.contains("|| true")
                || run.contains("exit 0")
                || (pwsh_test
                    && !run.trim_end().ends_with(
                        "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
                    ))
        })
}
