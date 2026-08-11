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

#[test]
fn routing_measurement_cli_rejects_every_same_id_frozen_corpus_tuple_mutation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&results, serde_json::to_vec(&results_value("high", false))?)?;

    let mut cases = vec![
        CORPUS.replacen("Add one mutation test", "Change the exact prompt", 1),
        CORPUS.replacen("The test is faithful and bounded.", "Changed oracle.", 1),
        CORPUS.replacen("simple-local-validator", "same-id-but-changed", 1),
        CORPUS.replacen("\"classification\":\"simple\"", "\"classification\":\"general\"", 1),
    ];
    let mut extra = serde_json::from_str::<Value>(CORPUS)?;
    extra["tasks"].as_array_mut().expect("tasks").push(json!({
        "id":"extra-task",
        "classification":"simple",
        "prompt":"Extra task.",
        "acceptance_oracle":"Extra oracle."
    }));
    cases.push(serde_json::to_string(&extra)?);
    let mut reordered = serde_json::from_str::<Value>(CORPUS)?;
    reordered["tasks"].as_array_mut().expect("tasks").reverse();
    cases.push(serde_json::to_string(&reordered)?);
    let mut missing = serde_json::from_str::<Value>(CORPUS)?;
    missing["tasks"].as_array_mut().expect("tasks").pop();
    cases.push(serde_json::to_string(&missing)?);
    let mut duplicate = serde_json::from_str::<Value>(CORPUS)?;
    let repeated = duplicate["tasks"][0].clone();
    duplicate["tasks"].as_array_mut().expect("tasks").push(repeated);
    cases.push(serde_json::to_string(&duplicate)?);

    for mutated in cases {
        fs::write(&corpus, mutated)?;
        assert!(
            !run(&corpus, &results)?.status.success(),
            "same-id mutated frozen corpus was accepted"
        );
    }
    fs::write(&corpus, CORPUS)?;
    assert!(run(&corpus, &results)?.status.success());
    let mut wrong_result = results_value("high", false);
    wrong_result["results"][0]["task_id"] = json!("wrong-task-id");
    fs::write(&results, serde_json::to_vec(&wrong_result)?)?;
    assert!(!run(&corpus, &results)?.status.success(), "wrong result task id passed");
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
        ("simple-local-validator", "Add one mutation test without editing production code.", "The test is faithful and bounded."),
        ("general-routing-contract", "Map the routing contract and return a minimal proof plan.", "The plan preserves current Terra/high."),
        ("ambiguous-specialist-boundary", "Classify an ownership-sensitive routing change and select the safe handler.", "The result fails closed without Luna."),
    ];
    let results = ["high", "xhigh", "max"]
        .into_iter()
        .flat_map(|thinking| {
            prompts.iter().map(move |(task_id, prompt, acceptance_oracle)| {
                json!({
                    "task_id": task_id,
                    "prompt": prompt,
                    "acceptance_oracle": acceptance_oracle,
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
