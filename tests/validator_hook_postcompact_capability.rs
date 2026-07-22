use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn pinned_build_records_postcompact_as_semantic_default_without_a_fallback() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let record: Value = serde_json::from_slice(&std::fs::read(
        root.join("plugins/codexy/hooks/postcompact-capability.json"),
    )?)?;
    assert_eq!(record["supportedCodexBuild"], json!("0.144.4"));
    assert_eq!(record["selection"]["semanticDefault"], json!("PostCompact"));
    assert_eq!(record["selection"]["selectedContextEvent"], json!("none"));
    for event in ["preCompact", "postCompact"] {
        assert_eq!(record[event]["modelVisibleDeveloperContext"], json!(false));
        assert_eq!(record[event]["manualTriggerSupported"], json!(true));
        assert_eq!(record[event]["automaticTriggerSupported"], json!(true));
        assert_eq!(record[event]["liveModelContextDeliveryProven"], json!(false));
        assert_eq!(record[event]["systemMessageCountsAsModelContext"], json!(false));
        assert_eq!(record[event]["upstreamTracker"], json!("https://github.com/eunsoogi/codexy/issues/455"));
    }
    Ok(())
}
