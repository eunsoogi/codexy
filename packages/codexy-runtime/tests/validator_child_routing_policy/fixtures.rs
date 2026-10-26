//! Shared request fixtures and library/CLI assertions for the routing requirements.

use crate::support::TestResult;
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

pub(super) const LUNA: (&str, &str) = ("gpt-6-luna", "max");
pub(super) const SOL: (&str, &str) = ("gpt-6.1-sol", "medium");

pub(super) fn hard_work_cases() -> TestResult<Vec<Value>> {
    [
        r#"{"criterion":"coupled_contracts","boundaries":["request parser","admission hook","delivery hook"],"coupling":"All three must enforce the same authenticated assignment"}"#,
        r#"{"criterion":"algorithm_design","problem":"Design a dependency scheduler","constraints":["Cycles must be reported","Ready nodes must preserve capacity"],"correctness_obligation":"Prove each node executes once after dependencies finish"}"#,
        r#"{"criterion":"cross_boundary_diagnosis","boundaries":["host","hook"],"competing_hypotheses":["Host dropped the field","Hook interpreted a stale envelope"],"observations":["Request includes assigned model","Delivery trace reports another model"]}"#,
    ].into_iter().map(|text| Ok(serde_json::from_str(text)?)).collect()
}

pub(super) fn request(classification: &str, pair: (&str, &str)) -> Value {
    json!({
        "schema":"codexy.child-routing-request.v1", "classification":classification,
        "simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,
            "low_risk_reversible":true,"no_unresolved_decision":true},
        "codex_thread_operation":"create_thread",
        "codex_thread_capabilities":{"models":[{"model":pair.0,"thinking":[pair.1]}]}
    })
}

pub(super) fn bounded_request() -> Value {
    let mut bounded = request("bounded_evidence", LUNA);
    bounded["simple_predicates"]["deterministic_oracle"] = json!(false);
    bounded["simple_predicates"]["bounded_evidence"] = json!({
        "read_only":true,"supplied_inputs_only":true,"input_count":2,
        "input_char_count":8_000,"output_item_limit":10
    });
    bounded
}

pub(super) fn fallback() -> Value {
    json!({"route":"root_or_named_specialist"})
}

pub(super) fn selected(route: &str, operation: &str, pair: (&str, &str)) -> Value {
    json!({"route":route,"codex_thread_operation":operation,"model":pair.0,"thinking":pair.1})
}

pub(super) fn root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}

pub(super) fn check(request: Value, expected: Value) -> TestResult {
    assert_eq!(resolve(&request)?, expected, "request: {request}");
    let output = cli(&request)?;
    assert!(
        output.status.success(),
        "resolver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "successful resolver wrote stderr");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

pub(super) fn rejects(request: Value, message: &str) -> TestResult {
    let error = resolve(&request).expect_err("invalid routing request passed");
    assert!(
        error.to_string().contains(message),
        "unexpected error: {error}"
    );
    let output = cli(&request)?;
    assert!(
        !output.status.success(),
        "invalid routing request passed CLI"
    );
    assert!(output.stdout.is_empty(), "invalid resolver wrote stdout");
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    Ok(())
}

pub(super) fn resolve(request: &Value) -> TestResult<Value> {
    Ok(codexy_runtime::validation::resolve_child_routing(
        &root(),
        &serde_json::to_string(request)?,
    )?)
}

pub(super) fn cli(request: &Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--plugin-root")
        .arg(root())
        .args(["--resolve-child-routing", "--routing-request-file"])
        .arg(request_path)
        .output()?)
}
