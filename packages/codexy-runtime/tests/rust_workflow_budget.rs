use serde_yaml::{Mapping, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const REQUIRED_JOBS: [(&str, &str, &str); 2] = [
    ("rust-test", "Ubuntu", "ubuntu-latest"),
    ("windows-rust-test", "Windows", "windows-latest"),
];
const REQUIRED_TARGETS: [&str; 13] = [
    "--lib --bins",
    "--test suite_support",
    "--test suite_agent",
    "--test suite_child",
    "--test suite_orchestration",
    "--test suite_governance",
    "--test suite_governance_workflows",
    "--test suite_system",
    "--test suite_runtime",
    "--test suite_runtime_activation",
    "--test suite_runtime_activation_autocrlf",
    "--test suite_sync_version",
    "--test suite_archive",
];
const CARGO_COMMAND: &str = "cargo test --manifest-path packages/codexy-runtime/Cargo.toml --locked ${{ matrix.target.args }}";
const FORBIDDEN_WORKFLOW_FRAGMENTS: [&str; 15] = [
    "|| true",
    "exit 0",
    "--ignored",
    "--skip",
    "retry",
    "sleep",
    "cargo fetch",
    "profiler",
    "receipt",
    "telemetry",
    "aggregate",
    "measure-command",
    "/usr/bin/time",
    "time cargo",
    "get-date",
];

#[test]
fn rust_workflow_has_exact_fail_closed_five_minute_matrix_per_platform() -> TestResult {
    let workflow = workflow_text()?;
    let failures = workflow_failures(&workflow)?;
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn rust_workflow_rejects_matrix_include_that_adds_a_duplicate_target() -> TestResult {
    assert_rejected(
        matrix_modifier_fixture("include", "duplicate system", "--test suite_system")?,
        "matrix.include duplicate target",
    )
}

#[test]
fn rust_workflow_rejects_matrix_exclude_that_removes_a_required_target() -> TestResult {
    assert_rejected(
        matrix_modifier_fixture("exclude", "system suite", "--test suite_system")?,
        "matrix.exclude required target",
    )
}

#[test]
fn rust_workflow_rejects_obvious_shell_success_masking() -> TestResult {
    let mut accepted = Vec::new();
    for suffix in [" || :", " || echo masked", "; true"] {
        let fixture = workflow_text()?.replacen(
            &format!("      - run: {CARGO_COMMAND}"),
            &format!("      - run: |\n          {CARGO_COMMAND}{suffix}"),
            1,
        );
        if workflow_failures(&fixture)?.is_empty() {
            accepted.push(suffix);
        }
    }
    assert!(accepted.is_empty(), "validator accepted {accepted:?}");
    Ok(())
}

fn workflow_text() -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/rust-test.yml"),
    )?)
}

fn workflow_failures(workflow: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let document: Value = serde_yaml::from_str(&workflow)?;
    let jobs = mapping_field(document.as_mapping(), "jobs", "workflow")?;
    let mut failures = Vec::new();
    for job_id in jobs.keys().filter_map(Value::as_str) {
        if !REQUIRED_JOBS
            .iter()
            .any(|(required, _, _)| *required == job_id)
        {
            failures.push(format!("workflow contains unexpected job {job_id}"));
        }
    }
    let normalized = workflow.to_ascii_lowercase();
    for fragment in FORBIDDEN_WORKFLOW_FRAGMENTS {
        if normalized.contains(fragment) {
            failures.push(format!("workflow contains forbidden fragment {fragment}"));
        }
    }

    for (job_id, platform, runner) in REQUIRED_JOBS {
        let job = mapping_field(Some(jobs), job_id, "jobs")?;
        if job.get("runs-on").and_then(Value::as_str) != Some(runner) {
            failures.push(format!("{platform} job must run on {runner}"));
        }
        if job.get("timeout-minutes").and_then(Value::as_u64) != Some(5) {
            failures.push(format!("{platform} job is missing timeout-minutes: 5"));
        }
        match matrix_target_args(job) {
            Ok(actual) => validate_target_union(platform, &actual, &mut failures),
            Err(error) => failures.push(format!("{platform} {error}")),
        }
        let cargo_steps = cargo_test_steps(job).collect::<Vec<_>>();
        if cargo_steps.len() != 1 {
            failures.push(format!(
                "{platform} job has {} cargo test step templates; expected 1",
                cargo_steps.len()
            ));
        } else {
            let run = cargo_steps[0];
            if !run.contains("--locked") {
                failures.push(format!("{platform} cargo test is not locked"));
            }
            if !run.contains("${{ matrix.target.args }}") {
                failures.push(format!("{platform} cargo test misses target matrix"));
            }
            if run.contains("--all-targets") {
                failures.push(format!("{platform} cargo test aggregates targets"));
            }
            let expected = if platform == "Windows" {
                format!("{CARGO_COMMAND}; if ($LASTEXITCODE -ne 0) {{ exit $LASTEXITCODE }}")
            } else {
                CARGO_COMMAND.to_owned()
            };
            if run.trim() != expected {
                failures.push(format!("{platform} cargo test command is not exact"));
            }
        }
        if weakens_failure_propagation(job) {
            failures.push(format!("{platform} job weakens failure propagation"));
        }
    }

    Ok(failures)
}

fn matrix_modifier_fixture(
    modifier: &str,
    name: &str,
    args: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let insertion = format!(
        "        {modifier}:\n          - target:\n              name: {name}\n              args: {args}\n    steps:"
    );
    Ok(workflow_text()?.replacen("    steps:", &insertion, 1))
}

fn assert_rejected(workflow: String, case: &str) -> TestResult {
    let failures = workflow_failures(&workflow)?;
    assert!(!failures.is_empty(), "validator accepted {case}");
    Ok(())
}

fn validate_target_union(platform: &str, actual: &[&str], failures: &mut Vec<String>) {
    for required in REQUIRED_TARGETS {
        let count = actual.iter().filter(|target| **target == required).count();
        if count != 1 {
            failures.push(format!(
                "{platform} target {required} appears {count} times; expected 1"
            ));
        }
    }
    for target in actual {
        if !REQUIRED_TARGETS.contains(target) {
            failures.push(format!("{platform} has unexpected target {target}"));
        }
    }
}

fn matrix_target_args(job: &Mapping) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let strategy = mapping_field(Some(job), "strategy", "job")?;
    let matrix = mapping_field(Some(strategy), "matrix", "strategy")?;
    if matrix.len() != 1 {
        return Err("matrix must contain only the exact target axis".into());
    }
    let targets = matrix
        .get("target")
        .and_then(Value::as_sequence)
        .ok_or("matrix missing target sequence")?;
    targets
        .iter()
        .map(|target| {
            target
                .as_mapping()
                .and_then(|target| target.get("args"))
                .and_then(Value::as_str)
                .ok_or_else(|| "matrix target missing args".into())
        })
        .collect()
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
    step_mappings(job)
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .filter(|run| run.contains("cargo test"))
}

fn weakens_failure_propagation(job: &Mapping) -> bool {
    job.contains_key("continue-on-error")
        || step_mappings(job).any(|step| {
            let run = step.get("run").and_then(Value::as_str).unwrap_or_default();
            let pwsh_test = step.get("shell").and_then(Value::as_str) == Some("pwsh")
                && run.contains("cargo test");
            step.contains_key("continue-on-error")
                || run.contains("|| true")
                || run.contains("exit 0")
                || (pwsh_test
                    && !run
                        .trim_end()
                        .ends_with("if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }"))
        })
}

fn step_mappings(job: &Mapping) -> impl Iterator<Item = &Mapping> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_mapping)
}
