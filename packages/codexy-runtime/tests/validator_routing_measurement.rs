use std::{fs, process::Command};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CORPUS: &str = r#"{
  "schema": "codexy.routing-evaluation-corpus.v1",
  "corpus_id": "routing-549-v1",
  "tasks": [
    {"id":"simple-local-validator","classification":"simple","prompt":"Add one mutation test without editing production code.","acceptance_oracle":"The test is faithful and bounded."},
    {"id":"general-routing-contract","classification":"general","prompt":"Map the routing contract and return a minimal proof plan.","acceptance_oracle":"The plan preserves current Terra/high."},
    {"id":"ambiguous-specialist-boundary","classification":"ambiguous","prompt":"Classify an ownership-sensitive routing change and select the safe handler.","acceptance_oracle":"The result fails closed without Luna."}
  ]
}"#;

#[test]
fn routing_measurement_cli_requires_paired_closed_results_and_fail_closed_selection() -> TestResult {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&corpus, CORPUS)?;
    fs::write(&results, serde_json::to_vec(&results_value("high", false))?)?;

    let valid = run(&corpus, &results)?;
    assert!(
        valid.status.success(),
        "valid paired high result failed: {}",
        String::from_utf8_lossy(&valid.stderr)
    );

    fs::write(&results, serde_json::to_vec(&results_value("max", true))?)?;
    let invalid = run(&corpus, &results)?;
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("must retain Terra/high"));
    Ok(())
}

#[test]
fn routing_measurement_cli_fails_closed_when_high_baseline_metrics_are_unavailable() -> TestResult {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&corpus, CORPUS)?;
    let mut value = results_value("xhigh", false);
    let observations = value["results"].as_array_mut().expect("results array");
    for observation in observations {
        if observation["thinking"] == "high" {
            observation["observed_cost_usd"] = Value::Null;
            if observation["task_id"] == "simple-local-validator" {
                observation["acceptance"] = json!("fail");
            }
        }
    }
    fs::write(&results, serde_json::to_vec(&value)?)?;

    let output = run(&corpus, &results)?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must retain Terra/high"), "stderr: {stderr}");
    Ok(())
}

#[test]
fn routing_measurement_cli_rejects_metrics_that_violate_the_closed_schema() -> TestResult {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&corpus, CORPUS)?;
    let mut value = results_value("high", false);
    value["results"][0]["tokens"] = json!(1.5);
    fs::write(&results, serde_json::to_vec(&value)?)?;

    let output = run(&corpus, &results)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("must contain one closed Terra observation"));
    Ok(())
}

#[test]
fn routing_measurement_cli_bounds_counts_and_requires_the_canonical_corpus() -> TestResult {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&corpus, CORPUS)?;
    let mut maximum = results_value("high", false);
    for result in maximum["results"].as_array_mut().expect("results array") {
        result["repairs_retries"] = json!(u32::MAX);
    }
    fs::write(&results, serde_json::to_vec(&maximum)?)?;
    let maximum_output = run(&corpus, &results)?;
    assert!(
        maximum_output.status.success(),
        "u32 maximum must remain fail-closed without crashing: {}",
        String::from_utf8_lossy(&maximum_output.stderr)
    );

    let mut oversized = maximum.clone();
    oversized["results"][0]["repairs_retries"] = json!(u64::from(u32::MAX) + 1);
    fs::write(&results, serde_json::to_vec(&oversized)?)?;
    assert!(!run(&corpus, &results)?.status.success(), "maximum-plus-one passed");
    let mut malformed = maximum.clone();
    malformed["results"][0]["repairs_retries"] = json!("not-an-integer");
    fs::write(&results, serde_json::to_vec(&malformed)?)?;
    assert!(!run(&corpus, &results)?.status.success(), "malformed count passed");

    let wrong_corpus = CORPUS.replace("routing-549-v1", "routing-other");
    fs::write(&corpus, &wrong_corpus)?;
    let mut stale = results_value("high", false);
    stale["corpus_id"] = json!("routing-other");
    fs::write(&results, serde_json::to_vec(&stale)?)?;
    assert!(!run(&corpus, &results)?.status.success(), "wrong corpus identity passed");
    Ok(())
}

fn run(corpus: &std::path::Path, results: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-routing-measurement", "--routing-corpus-file"])
        .arg(corpus)
        .arg("--routing-results-file")
        .arg(results)
        .output()
}

fn results_value(selected_effort: &str, omit_cost: bool) -> Value {
    let prompts = [
        ("simple-local-validator", "Add one mutation test without editing production code."),
        ("general-routing-contract", "Map the routing contract and return a minimal proof plan."),
        ("ambiguous-specialist-boundary", "Classify an ownership-sensitive routing change and select the safe handler."),
    ];
    let results = ["high", "xhigh", "max"]
        .into_iter()
        .flat_map(|thinking| {
            prompts.iter().map(move |(task_id, prompt)| {
                json!({
                    "task_id": task_id,
                    "prompt": prompt,
                    "model": "gpt-5.6-terra",
                    "thinking": thinking,
                    "acceptance": "pass",
                    "p0_p1_misses": 0,
                    "proof_complete": true,
                    "repairs_retries": 0,
                    "tokens": 100,
                    "wall_time_ms": 1000,
                    "observed_cost_usd": if omit_cost { Value::Null } else { json!(0.01) }
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "codexy.routing-evaluation-results.v1",
        "corpus_id": "routing-549-v1",
        "selected_effort": selected_effort,
        "results": results
    })
}
