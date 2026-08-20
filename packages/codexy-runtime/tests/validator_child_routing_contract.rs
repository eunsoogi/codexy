use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

const POLICY: &str = "skills/orchestration/references/child-routing-policy.json";
const RESULTS: &str = "skills/orchestration/references/routing-evaluation-results.json";

#[test]
fn resolver_prefers_luna_before_terra_for_every_generic_route() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY), Path::new(RESULTS)])?;
    let luna = json!({"model":"gpt-5.6-luna","thinking":["max"]});
    let luna_high = json!({"model":"gpt-5.6-luna","thinking":["high"]});
    let terra = json!({"model":"gpt-5.6-terra","thinking":["high"]});
    let terra_max = json!({"model":"gpt-5.6-terra","thinking":["max"]});
    let predicates = json!({
        "fixed_scope": true,
        "deterministic_oracle": true,
        "low_risk_reversible": true,
        "no_unresolved_decision": true
    });

    for operation in ["create_thread", "send_message_to_thread"] {
        for (classification, simple_predicates, models, expected) in [
            (
                "general",
                Value::Null,
                json!([luna.clone(), terra.clone()]),
                json!({"model":"gpt-5.6-luna","thinking":"max"}),
            ),
            (
                "general",
                Value::Null,
                json!([luna.clone()]),
                json!({"model":"gpt-5.6-luna","thinking":"max"}),
            ),
            (
                "general",
                Value::Null,
                json!([terra.clone()]),
                json!({"model":"gpt-5.6-terra","thinking":"high"}),
            ),
            (
                "general",
                Value::Null,
                json!([luna_high.clone(), terra.clone()]),
                json!({"model":"gpt-5.6-terra","thinking":"high"}),
            ),
            (
                "general",
                Value::Null,
                json!([terra_max.clone()]),
                Value::Null,
            ),
            ("general", Value::Null, json!([]), Value::Null),
            (
                "simple",
                predicates.clone(),
                json!([luna.clone(), terra.clone()]),
                json!({"model":"gpt-5.6-luna","thinking":"max"}),
            ),
            (
                "simple",
                predicates.clone(),
                json!([terra.clone()]),
                json!({"model":"gpt-5.6-terra","thinking":"high"}),
            ),
            ("simple", predicates.clone(), json!([]), Value::Null),
        ] {
            let mut request = json!({
                "schema":"codexy.child-routing-request.v1",
                "classification":classification,
                "codex_thread_operation":operation,
                "codex_thread_capabilities":{"models":models}
            });
            if !simple_predicates.is_null() {
                request["simple_predicates"] = simple_predicates;
            }
            let expected = if expected.is_null() {
                json!({"route":"root_or_named_specialist"})
            } else {
                json!({
                    "route":"generic",
                    "codex_thread_operation":operation,
                    "model":expected["model"],
                    "thinking":expected["thinking"]
                })
            };
            assert_route(fixture.root(), request, expected)?;
        }
    }

    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "named_specialist":"codexy-architect",
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[luna, terra]}
        }),
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    Ok(())
}

fn assert_route(root: &Path, request: Value, expected: Value) -> TestResult {
    let output = resolve(root, request)?;
    assert!(output.status.success(), "resolver failed: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn resolve(root: &Path, request: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--resolve-child-routing", "--routing-request-file"])
        .arg(request_path)
        .output()?)
}
