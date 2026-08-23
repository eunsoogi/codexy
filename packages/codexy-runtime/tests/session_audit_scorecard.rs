use std::{fs, process::Command};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn controlled_scorecard_reports_typed_aggregate_coverage() -> TestResult {
    let scorecard = fixture()?;
    let output = validate(&scorecard)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let result: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(result["valid"], true);
    assert_eq!(result["comparisonCount"], 7);
    assert_eq!(result["taskClassCount"], 5);
    assert_eq!(result["ownerKindCount"], 3);
    assert_eq!(result["phaseCount"], 7);
    Ok(())
}

#[test]
fn scorecard_rejects_noncomparable_windows_and_synthesized_measures() -> TestResult {
    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]["window"]["afterRecords"] = json!(7);
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("equal record-count"));

    let mut scorecard = fixture()?;
    scorecard["measureAvailability"]["inputTokens"] = json!("unavailable");
    scorecard["comparisons"][0]["before"]["inputTokens"] = json!(100);
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unavailable measure must remain null"));
    Ok(())
}

#[test]
fn scorecard_rejects_weakened_gates_and_private_nested_content() -> TestResult {
    let mut scorecard = fixture()?;
    scorecard["thresholds"]["acceptanceMinPct"] = json!(94.9);
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("acceptance floors"));

    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]
        .as_object_mut()
        .ok_or("comparison must be an object")?
        .insert("prompt".into(), json!("private-marker"));
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    let error = stderr(&output);
    assert!(error.contains("closed schema"));
    assert!(!error.contains("private-marker"));

    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]["phase"] = json!("private-enum-marker");
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(!stderr(&output).contains("private-enum-marker"));
    Ok(())
}

#[test]
fn scorecard_enforces_outcomes_complete_coverage_and_required_nulls() -> TestResult {
    let mut scorecard = fixture()?;
    for comparison in scorecard["comparisons"]
        .as_array_mut()
        .ok_or("comparisons must be an array")?
    {
        comparison["after"]["inputTokens"] = comparison["before"]["inputTokens"].clone();
    }
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("every scorecard threshold"));

    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]["after"]["p0P1Misses"] = json!(1);
    assert!(!validate(&scorecard)?.status.success());

    let mut scorecard = fixture()?;
    for comparison in scorecard["comparisons"]
        .as_array_mut()
        .ok_or("comparisons must be an array")?
    {
        comparison["after"]["toolOutputBytes"] =
            comparison["before"]["toolOutputBytes"].clone();
    }
    assert!(!validate(&scorecard)?.status.success());

    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]["after"]["acceptedRuns"] = json!(0);
    scorecard["comparisons"][0]["after"]["proofCompleteRuns"] = json!(0);
    assert!(!validate(&scorecard)?.status.success());

    let mut scorecard = fixture()?;
    scorecard["comparisons"][0]["after"]["repairs"] = json!(1);
    assert!(!validate(&scorecard)?.status.success());

    let mut scorecard = fixture()?;
    let orphan = scorecard["comparisons"][0].clone();
    scorecard["comparisons"]
        .as_array_mut()
        .ok_or("comparisons must be an array")?
        .push(orphan);
    scorecard["comparisons"][7]["id"] = json!("orphan");
    assert!(!validate(&scorecard)?.status.success());

    for name in [
        "inputTokens",
        "wallTimeMs",
        "observedCostUsd",
        "toolInputBytes",
        "toolOutputBytes",
        "cacheInputTokens",
    ] {
        let mut scorecard = fixture()?;
        scorecard["comparisons"][0]["before"]
            .as_object_mut()
            .ok_or("measurements must be an object")?
            .remove(name);
        assert!(!validate(&scorecard)?.status.success(), "missing {name}");
    }
    Ok(())
}

#[test]
fn integrated_optimization_sets_support_independent_decisions() -> TestResult {
    let mut scorecard = fixture()?;
    for comparison in scorecard["comparisons"]
        .as_array_mut()
        .ok_or("comparisons must be an array")?
    {
        comparison["optimizationSet"] = json!(["baseline", "integrated"]);
    }
    let mut decision = scorecard["decisionInputs"][0].clone();
    decision["optimizationId"] = json!("integrated");
    scorecard["decisionInputs"]
        .as_array_mut()
        .ok_or("decision inputs must be an array")?
        .push(decision);
    let output = validate(&scorecard)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn scorecard_requires_the_representative_task_corpus() -> TestResult {
    let mut scorecard = fixture()?;
    scorecard["comparisons"]
        .as_array_mut()
        .ok_or("comparisons must be an array")?
        .remove(0);
    let output = validate(&scorecard)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("representative corpus"));
    Ok(())
}

fn fixture() -> TestResult<Value> {
    Ok(serde_json::from_str(include_str!(
        "fixtures/session-audit/controlled-scorecard.json"
    ))?)
}

fn validate(scorecard: &Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("scorecard.json");
    fs::write(&path, serde_json::to_vec(scorecard)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--scorecard")
        .arg(path)
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
