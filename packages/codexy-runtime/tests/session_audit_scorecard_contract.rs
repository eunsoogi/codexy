use std::{fs, process::Command};

use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn packaged_scorecard_schema_corpus_and_baseline_are_closed_and_consistent() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let references = root.join("plugins/codexy/skills/orchestration/references");
    let schema: Value = serde_json::from_slice(&fs::read(
        references.join("efficiency-scorecard.schema.json"),
    )?)?;
    let corpus: Value = serde_json::from_slice(&fs::read(
        references.join("efficiency-scorecard-corpus.json"),
    )?)?;
    let baseline_path = references.join("efficiency-scorecard-baseline.json");
    let baseline: Value = serde_json::from_slice(&fs::read(&baseline_path)?)?;

    assert_eq!(schema["properties"]["schema"]["const"], "codexy.efficiency-scorecard.v1");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(corpus["tasks"].as_array().map(Vec::len), Some(5));
    assert_eq!(baseline["corpusId"], corpus["corpusId"]);

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--scorecard")
        .arg(baseline_path)
        .output()?;
    assert!(output.status.success(), "stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

#[test]
fn packaged_baseline_keeps_unavailable_runtime_measures_null() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let path = root.join(
        "plugins/codexy/skills/orchestration/references/efficiency-scorecard-baseline.json",
    );
    let baseline: Value = serde_json::from_slice(&fs::read(path)?)?;
    for comparison in baseline["comparisons"]
        .as_array()
        .ok_or("comparisons must be an array")?
    {
        for side in ["before", "after"] {
            for metric in [
                "inputTokens",
                "wallTimeMs",
                "observedCostUsd",
                "cacheInputTokens",
            ] {
                assert!(comparison[side][metric].is_null(), "{side}.{metric} must stay null");
            }
        }
    }
    Ok(())
}
