use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

const CORPUS: &str = r#"{
  "schema":"codexy.routing-evaluation-corpus.v1",
  "corpus_id":"routing-600-v1",
  "tasks":[
    {"id":"simple-local-validator","classification":"simple","prompt":"Add one mutation test without editing production code.","acceptance_oracle":"The candidate adds exactly one mutation test without editing production code."},
    {"id":"general-routing-contract","classification":"general","prompt":"Resolve a generic child route across the callable model surface.","acceptance_oracle":"Luna/max wins whenever callable; Terra/high is availability fallback."},
    {"id":"ambiguous-specialist-boundary","classification":"ambiguous","prompt":"Classify an ownership-sensitive routing change and select the safe handler.","acceptance_oracle":"The result fails closed without a callable generic route."}
  ]
}"#;

#[test]
fn promotion_selects_xhigh_when_both_candidates_are_viable() -> TestResult {
    let mut result = results("xhigh");
    set(&mut result, "high", "simple-local-validator", "acceptance", json!("fail"));
    set(&mut result, "max", "simple-local-validator", "wall_time_ms", json!(151));
    set(&mut result, "max", "simple-local-validator", "observed_cost_usd", json!(1.51));
    assert_success(&result)?;
    result["selected_effort"] = json!("max");
    assert_failure(&result, "expected xhigh")
}

#[test]
fn promotion_selects_max_when_xhigh_is_not_viable() -> TestResult {
    let mut result = results("max");
    set(&mut result, "high", "simple-local-validator", "acceptance", json!("fail"));
    set(&mut result, "xhigh", "simple-local-validator", "p0_p1_misses", json!(1));
    assert_success(&result)
}

#[test]
fn promotion_rejects_final_only_acceptance_gain_after_retries() -> TestResult {
    let mut result = results("xhigh");
    set(&mut result, "high", "simple-local-validator", "acceptance", json!("fail"));
    for effort in ["xhigh", "max"] {
        for task in [
            "simple-local-validator",
            "general-routing-contract",
            "ambiguous-specialist-boundary",
        ] {
            set(&mut result, effort, task, "repairs_retries", json!(1));
        }
    }
    assert_failure(&result, "expected high")
}

#[test]
fn promotion_boundaries_require_quality_proof_and_economics() -> TestResult {
    let mut at_limit = results("xhigh");
    set(&mut at_limit, "high", "simple-local-validator", "acceptance", json!("fail"));
    set(&mut at_limit, "max", "simple-local-validator", "p0_p1_misses", json!(1));
    for metric in ["wall_time_ms", "observed_cost_usd"] {
        for task in ["simple-local-validator", "general-routing-contract", "ambiguous-specialist-boundary"] {
            set(&mut at_limit, "xhigh", task, metric, if metric == "wall_time_ms" { json!(150) } else { json!(1.5) });
        }
    }
    assert_success(&at_limit)?;
    let mut repair_boundary = results("xhigh");
    set(&mut repair_boundary, "high", "simple-local-validator", "repairs_retries", json!(5));
    set(&mut repair_boundary, "xhigh", "simple-local-validator", "repairs_retries", json!(4));
    assert_success(&repair_boundary)?;

    for (field, value) in [
        ("p0_p1_misses", json!(1)),
        ("proof_complete", json!(false)),
        ("acceptance", json!("fail")),
        ("wall_time_ms", json!(151)),
        ("observed_cost_usd", json!(1.51)),
    ] {
        let mut rejected = at_limit.clone();
        for task in [
            "simple-local-validator",
            "general-routing-contract",
            "ambiguous-specialist-boundary",
        ] {
            set(&mut rejected, "xhigh", task, field, value.clone());
        }
        assert_failure(&rejected, "expected high").map_err(|error| format!("{field}: {error}"))?;
    }
    Ok(())
}

#[test]
fn results_reject_stale_identity_duplicates_and_unknown_fields() -> TestResult {
    let mut stale = results("high");
    stale["corpus_id"] = json!("stale");
    assert_failure(&stale, "must bind the frozen corpus identity")?;
    let mut duplicate = results("high");
    let repeated = duplicate["results"][0].clone();
    duplicate["results"].as_array_mut().unwrap().push(repeated);
    assert_failure(&duplicate, "must contain one closed Terra observation")?;
    let mut wrong_prompt = results("high");
    wrong_prompt["results"][0]["prompt"] = json!("stale prompt");
    assert_failure(&wrong_prompt, "must contain one closed Terra observation")?;
    let mut wrong_oracle = results("high");
    wrong_oracle["results"][0]["acceptance_oracle"] = json!("stale oracle");
    assert_failure(&wrong_oracle, "must contain one closed Terra observation")?;
    let mut extra = results("high");
    extra["results"][0]["unexpected"] = json!(true);
    assert_failure(&extra, "must be closed typed JSON")
}

fn results(selected: &str) -> Value {
    let tasks = [
        ("simple-local-validator", "Add one mutation test without editing production code.", "The candidate adds exactly one mutation test without editing production code."),
        ("general-routing-contract", "Resolve a generic child route across the callable model surface.", "Luna/max wins whenever callable; Terra/high is availability fallback."),
        ("ambiguous-specialist-boundary", "Classify an ownership-sensitive routing change and select the safe handler.", "The result fails closed without a callable generic route."),
    ];
    let observations = ["high", "xhigh", "max"]
        .into_iter()
        .flat_map(|thinking| {
            tasks.iter().map(move |(task_id, prompt, acceptance_oracle)| {
                json!({
                    "task_id": task_id, "prompt": prompt,
                    "acceptance_oracle": acceptance_oracle,
                    "model": "gpt-5.6-terra", "thinking": thinking,
                    "acceptance": "pass", "p0_p1_misses": 0,
                    "proof_complete": true, "repairs_retries": 0,
                    "tokens": 100, "wall_time_ms": 100, "observed_cost_usd": 1.0
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "codexy.routing-evaluation-results.v1",
        "corpus_id": "routing-600-v1",
        "selected_effort": selected,
        "results": observations
    })
}

fn set(result: &mut Value, thinking: &str, task_id: &str, field: &str, value: Value) {
    let observation = result["results"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["thinking"] == thinking && item["task_id"] == task_id)
        .unwrap();
    observation[field] = value;
}

fn assert_success(result: &Value) -> TestResult {
    if run(result)?.status.success() { Ok(()) } else { Err("measurement unexpectedly failed".into()) }
}

fn assert_failure(result: &Value, expected: &str) -> TestResult {
    let output = run(result)?;
    if output.status.success() { return Err("measurement unexpectedly passed".into()); }
    if String::from_utf8_lossy(&output.stderr).contains(expected) {
        Ok(())
    } else {
        Err(format!("missing expected diagnostic {expected}").into())
    }
}

fn run(result: &Value) -> std::io::Result<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let corpus = temp.path().join("corpus.json");
    let results = temp.path().join("results.json");
    fs::write(&corpus, CORPUS)?;
    fs::write(&results, serde_json::to_vec(result)?)?;
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-routing-measurement", "--routing-corpus-file"])
        .arg(corpus)
        .arg("--routing-results-file")
        .arg(results)
        .output()
}
